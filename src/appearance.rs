use crate::{
    accent, config::Config, i18n, lockscreen, log, slideshow, state, theme, themes, MainWindow,
};
use slint::ComponentHandle;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
};
use theme::Theme;

#[derive(Default)]
pub struct Worker {
    generation: u64,
    queue: Option<Queue>,
}

struct Queue {
    pending: Arc<Mutex<Option<Job>>>,
    revision: Arc<AtomicU64>,
    wake: SyncSender<()>,
}

#[derive(PartialEq)]
struct Appearance {
    generation: u64,
    wallpaper: Option<PathBuf>,
    lockscreen: Option<PathBuf>,
    interval: u32,
    shuffle: bool,
    accent: Option<u32>,
}

#[derive(Clone, Copy)]
pub enum Trigger {
    Scheduled { switched: bool },
    Manual,
}

struct Job {
    appearance: Appearance,
    trigger: Trigger,
    lang: i18n::Lang,
    revision: u64,
}

#[derive(Default)]
struct Applied {
    generation: u64,
    wallpaper: Option<slideshow::AppliedWallpaper>,
    lockscreen: Option<PathBuf>,
    accent: Option<u32>,
}

impl Worker {
    pub fn invalidate(&mut self) {
        self.generation += 1;
        if let Some(queue) = &self.queue {
            let mut pending = queue.pending.lock().unwrap();
            queue.revision.fetch_add(1, Ordering::Relaxed);
            *pending = None;
        }
    }

    pub fn submit(
        &mut self,
        ui: &MainWindow,
        cfg: &Config,
        themes: &[themes::ThemeEntry],
        target: Theme,
        trigger: Trigger,
    ) {
        let appearance = Appearance {
            generation: self.generation,
            wallpaper: state::resolve_wallpaper_path(cfg, themes, target),
            lockscreen: state::resolve_lock_wallpaper_path(cfg, themes, target),
            interval: slideshow::clamp_interval(cfg.slideshow_interval_min),
            shuffle: cfg.slideshow_shuffle,
            accent: cfg
                .accent_enabled
                .then(|| {
                    accent::parse_hex(if target == Theme::Light {
                        &cfg.light_accent
                    } else {
                        &cfg.dark_accent
                    })
                })
                .flatten(),
        };

        if self.queue.is_none()
            && appearance.wallpaper.is_none()
            && appearance.lockscreen.is_none()
            && appearance.accent.is_none()
        {
            return;
        }

        let queue = self.queue.get_or_insert_with(|| Queue::start(ui.as_weak()));
        let mut pending = queue.pending.lock().unwrap();
        let trigger = merge_trigger(pending.as_ref(), &appearance, trigger);
        let revision = queue.revision.fetch_add(1, Ordering::Relaxed) + 1;
        *pending = Some(Job {
            appearance,
            trigger,
            lang: i18n::Lang::from_code(&cfg.language),
            revision,
        });
        let _ = queue.wake.try_send(());
    }
}

fn merge_trigger(previous: Option<&Job>, target: &Appearance, trigger: Trigger) -> Trigger {
    match (previous, trigger) {
        (Some(job), Trigger::Scheduled { switched: false }) if &job.appearance == target => {
            job.trigger
        }
        _ => trigger,
    }
}

impl Queue {
    fn start(ui: slint::Weak<MainWindow>) -> Self {
        let (wake, requests) = mpsc::sync_channel(1);
        let pending = Arc::new(Mutex::new(None::<Job>));
        let revision = Arc::new(AtomicU64::new(0));
        let (jobs, latest) = (pending.clone(), revision.clone());
        std::thread::spawn(move || {
            use windows::Win32::System::Com::{
                CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED,
            };

            let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let mut applied = Applied::default();
            while requests.recv().is_ok() {
                let job = jobs.lock().unwrap().take();
                if let Some(job) = job {
                    if let Some(status) = applied.apply(&job, &latest) {
                        let latest = latest.clone();
                        let _ = ui.upgrade_in_event_loop(move |ui| {
                            if latest.load(Ordering::Relaxed) == job.revision {
                                ui.set_status(status.into());
                            }
                        });
                    }
                }
            }
            if com.is_ok() {
                unsafe { CoUninitialize() };
            }
        });
        Self {
            pending,
            revision,
            wake,
        }
    }
}

impl Applied {
    fn apply(&mut self, job: &Job, latest: &AtomicU64) -> Option<String> {
        if self.generation != job.appearance.generation {
            *self = Self {
                generation: job.appearance.generation,
                ..Self::default()
            };
        }
        let current = || latest.load(Ordering::Relaxed) == job.revision;
        let mut status = None;
        let mut report = |key, err: anyhow::Error| {
            log::warn(format!("{key} failed: {err:#}"));
            status = Some(i18n::msg(job.lang, key, &format!("{err:#}")));
        };

        if current() {
            if let Some(color) = job
                .appearance
                .accent
                .filter(|color| Some(*color) != self.accent)
            {
                match accent::apply(color) {
                    Ok(()) => self.accent = Some(color),
                    Err(err) => report("accent", err),
                }
            }
        }
        if current() {
            if let Err(err) = self.apply_wallpaper(job) {
                report("wallpaper", err);
            }
        }
        if current() {
            if let Err(err) = self.apply_lockscreen(&job.appearance) {
                report("lockscreen", err);
            }
        }
        status
    }

    fn apply_wallpaper(&mut self, job: &Job) -> anyhow::Result<()> {
        let options = &job.appearance;
        let Some(path) = options.wallpaper.as_ref().filter(|path| path.exists()) else {
            return Ok(());
        };
        let key = slideshow::applied_key(path, options.interval, options.shuffle);
        match job.trigger {
            Trigger::Manual => slideshow::apply_explicit(path, options.interval, options.shuffle)?,
            Trigger::Scheduled { switched } => {
                let changed = self.wallpaper.as_ref() != Some(&key);
                match slideshow::plan_apply(path.is_dir(), changed, switched) {
                    slideshow::ApplyPlan::Skip => return Ok(()),
                    slideshow::ApplyPlan::SetSingle => theme::set_wallpaper(path)?,
                    slideshow::ApplyPlan::ConfigureShow => {
                        slideshow::set_slideshow(path, options.interval, options.shuffle)?
                    }
                    slideshow::ApplyPlan::AdvanceShow => slideshow::advance()?,
                }
            }
        }
        self.wallpaper = Some(key);
        Ok(())
    }

    fn apply_lockscreen(&mut self, options: &Appearance) -> anyhow::Result<()> {
        let Some(path) = options.lockscreen.as_ref().and_then(|path| {
            if path.is_dir() {
                slideshow::first_image(path)
            } else {
                Some(path.clone())
            }
        }) else {
            return Ok(());
        };
        if self.lockscreen.as_ref() != Some(&path) && path.exists() {
            lockscreen::set_image(&path)?;
            log::info(format!("lock screen: applied {}", path.display()));
            self.lockscreen = Some(path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance(generation: u64) -> Appearance {
        Appearance {
            generation,
            wallpaper: None,
            lockscreen: None,
            interval: 30,
            shuffle: false,
            accent: None,
        }
    }

    #[test]
    fn schedule_poll_preserves_pending_transition_only_for_same_appearance() {
        for trigger in [Trigger::Manual, Trigger::Scheduled { switched: true }] {
            let pending = Job {
                appearance: appearance(0),
                trigger,
                lang: i18n::Lang::En,
                revision: 1,
            };
            let poll = Trigger::Scheduled { switched: false };
            assert_eq!(
                std::mem::discriminant(&merge_trigger(Some(&pending), &appearance(0), poll)),
                std::mem::discriminant(&trigger),
            );
            assert!(!matches!(
                merge_trigger(Some(&pending), &appearance(0), poll),
                Trigger::Scheduled { switched: false }
            ));
            assert!(matches!(
                merge_trigger(Some(&pending), &appearance(1), poll),
                Trigger::Scheduled { switched: false }
            ));
        }
    }
}
