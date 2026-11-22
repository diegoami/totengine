//! Desktop presentation controls for a watched player seat.
use super::*;
use engine::human::SeatWatcher;

pub(super) struct DesktopWatch {
    driver: SeatWatcher,
    pub(super) running: bool,
}

impl Client {
    pub(super) fn watch(&self) -> Option<&DesktopWatch> {
        self.loaded.as_ref()?.watch.as_ref()
    }

    pub(super) fn watching_seat(&self) -> bool {
        self.watch().is_some()
    }

    pub(super) fn can_watch_step(&self) -> bool {
        !self.city_open
            && self.menu.is_none()
            && matches!(
                self.modal,
                None | Some(
                    Modal::Research
                        | Modal::Name
                        | Modal::Government { .. }
                        | Modal::Rates {
                            review: Some(_),
                            ..
                        }
                        | Modal::Talks
                )
            )
            && self.loaded.as_ref().is_some_and(|loaded| {
                matches!(loaded.session.phase(), Phase::Human | Phase::Opponents)
            })
    }

    pub(super) fn toggle_watched_seat(&mut self) -> Result<(), String> {
        let loaded = self.loaded.as_mut().ok_or("Start a game before watching")?;
        if loaded.watch.take().is_some() {
            self.advancing = false;
            self.status = "Stopped watching this civilization".into();
            return Ok(());
        }
        if !matches!(loaded.session.phase(), Phase::Human | Phase::Opponents) {
            return Err("This civilization has no turns left to watch".into());
        }
        loaded.watch = Some(DesktopWatch {
            driver: loaded.session.watch_seat()?,
            running: true,
        });
        self.status = "AI is playing this civilization".into();
        Ok(())
    }

    pub(super) fn set_watch_running(&mut self, running: bool) -> Result<(), String> {
        let watch = self
            .loaded
            .as_mut()
            .and_then(|l| l.watch.as_mut())
            .ok_or("Start watching first")?;
        watch.running = running;
        Ok(())
    }

    pub(super) fn pause_watched_seat(&mut self) -> Result<(), String> {
        let running = self.watch().is_some_and(|watch| watch.running);
        self.set_watch_running(!running)
    }

    pub(super) fn step_watched_seat(&mut self) -> Result<(), String> {
        let view = self.view().ok_or("Start a game before watching")?;
        if !matches!(view.phase, Phase::Human | Phase::Opponents) {
            return Err("This civilization has no turns left to watch".into());
        }
        if !self.can_watch_step() {
            return Err("Close inspection or the notice to continue watching".into());
        }
        if view.phase == Phase::Opponents {
            if let Some(loaded) = &mut self.loaded {
                loaded.session.advance(1);
            }
            self.refresh();
            if view.observation.turn != self.view().ok_or("Game unavailable")?.observation.turn {
                self.status = "New round".into();
            }
            self.required();
            return Ok(());
        }
        let watch = self
            .loaded
            .as_mut()
            .and_then(|l| l.watch.as_mut())
            .ok_or("Start watching first")?;
        let command = match watch.driver.command(&view) {
            Ok(command) => command,
            Err(error) => {
                watch.running = false;
                return Err(error.into());
            }
        };
        let actor = match command {
            Command::Move { unit, .. }
            | Command::Occupy { unit, .. }
            | Command::BeginFound { unit } => Some(unit),
            _ => None,
        };
        let decision_status = match command {
            Command::Research { advance, .. } => {
                Some(format!("AI chose {}", self.tech_name(advance)))
            }
            Command::Government { government, .. } => {
                Some(format!("AI chose {}", routes::government_name(government)))
            }
            Command::NameCity { name, .. } => Some(format!("AI founded {}", name.as_str())),
            _ => None,
        };
        if let Err(error) = self.apply_order(command) {
            if let Some(watch) = self.loaded.as_mut().and_then(|l| l.watch.as_mut()) {
                watch.driver.record_rejection();
                if view.decision.is_some() {
                    watch.running = false;
                }
            }
            return Err(error);
        }
        if view.decision.is_some() && self.view().is_some_and(|v| v.decision.is_none()) {
            self.modal = None;
        }
        if let Some(status) = decision_status {
            self.status = status;
        }
        if let Some(unit) = actor.filter(|unit| self.view().is_some_and(|v| v.units.contains(unit)))
        {
            self.active = Some(unit);
            self.inspect = self.active_position();
            self.center();
        }
        Ok(())
    }
}
