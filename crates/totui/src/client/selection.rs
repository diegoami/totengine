//! Presentation selection uses only the human view; never advances the session.
use super::*;
use engine::player::UnitView;

impl Client {
    pub(super) fn active_position(&self) -> Option<(u16, u16)> {
        let v = self.view()?;
        let e = self.active?;
        v.observation
            .units
            .iter()
            .find(|u| u.index == e.slot)
            .map(|u| (u.x, u.y))
    }

    /// Keep a ready actor, otherwise advance after its slot. Explicit end-turn
    /// remains the owner's default; selection itself never submits an order.
    pub(super) fn schedule_unit(&mut self, after: Option<Entity>, new_turn: bool) {
        let Some(v) = self.view() else { return };
        if v.phase != Phase::Human || v.decision.is_some() {
            self.moving = false;
            return;
        }
        let ready: Vec<_> = v
            .units
            .iter()
            .copied()
            .filter(|e| {
                v.observation
                    .units
                    .iter()
                    .any(|u| u.index == e.slot && u.moves_left > 0)
            })
            .collect();
        let old = self.active;
        if new_turn || !self.active.is_some_and(|e| ready.contains(&e)) {
            let next = after
                .and_then(|a| ready.iter().find(|e| e.slot > a.slot))
                .or(ready.first())
                .copied();
            // Keep an exhausted identity inspectable when no ready units remain.
            self.active = next.or_else(|| old.filter(|e| v.units.contains(e)));
        }
        self.moving = self.active.is_some_and(|e| ready.contains(&e));
        if self.moving || old != self.active {
            self.inspect = self.active_position();
            self.center();
        }
        self.focus_end_if_idle();
    }

    /// Make Enter's default action visible when returning to an exhausted map.
    /// Called on transitions only, so Tab can still choose another action.
    pub(super) fn focus_end_if_idle(&mut self) {
        if self.modal.is_some() || self.city_open {
            return;
        }
        let Some(v) = self.view() else { return };
        if v.phase == Phase::Human
            && v.decision.is_none()
            && !v
                .observation
                .units
                .iter()
                .any(|u| u.owner == v.observation.civ && u.moves_left > 0)
        {
            if let Some(i) = self
                .buttons()
                .iter()
                .position(|b| matches!(b.action, Action::End))
            {
                self.focus_button = i;
            }
        }
    }

    pub(super) fn select_at(&mut self, at: (u16, u16)) -> Result<(), String> {
        let v = self.view().ok_or("Start a game first")?;
        let units: Vec<_> = v
            .units
            .iter()
            .copied()
            .filter(|e| {
                v.observation
                    .units
                    .iter()
                    .any(|u| u.index == e.slot && (u.x, u.y) == at)
            })
            .collect();
        self.inspect = Some(at);
        match units.as_slice() {
            [unit] => self.action(Action::Unit(*unit)),
            [] => Err("No own unit at the inspection cursor".into()),
            _ => {
                self.open(Modal::Units { at, units });
                Ok(())
            }
        }
    }

    /// Draw exactly one unit per occupied tile. The selected member wins; the
    /// active stack is on top. The reverse of this same order is used for hits.
    pub(super) fn display_units<'a>(&self, v: &'a HumanView) -> Vec<&'a UnitView> {
        let mut tiles = BTreeMap::new();
        for u in &v.observation.units {
            let key = (u.y, u.x);
            if tiles
                .get(&key)
                .is_none_or(|old: &&UnitView| !self.active.is_some_and(|e| e.slot == old.index))
            {
                tiles.insert(key, u);
            }
        }
        let mut units: Vec<_> = tiles.into_values().collect();
        units.sort_by_key(|u| (self.active.is_some_and(|e| e.slot == u.index), u.y, u.x));
        units
    }

    pub(super) fn hit_own_city<'a>(
        &self,
        v: &'a HumanView,
        x: f64,
        y: f64,
    ) -> Option<&'a engine::player::CityView> {
        for c in v
            .observation
            .cities
            .iter()
            .rev()
            .filter(|c| c.owner == v.observation.civ)
        {
            let px = self.pan.0 + f64::from(c.x) * 32. * self.zoom;
            let py = self.pan.1 + f64::from(c.y) * 16. * self.zoom;
            if !(44.0..720.0).contains(&px) || !(150.0..640.0).contains(&py) {
                continue;
            }
            let sx = (x - (px - 32. * self.zoom)) / (64. * self.zoom);
            let sy = (y - (py - 34. * self.zoom)) / (48. * self.zoom);
            if !(0.0..1.0).contains(&sx) || !(0.0..1.0).contains(&sy) {
                continue;
            }
            let sprite = self
                .atlas()?
                .sprites
                .get(&format!("city-{}", u8::from(c.size > 4)))?;
            let col = (sx * f64::from(sprite.w)) as usize;
            let row = (sy * f64::from(sprite.h)) as usize;
            if sprite.rgba[(row * sprite.w as usize + col) * 4 + 3] > 0 {
                return Some(c);
            }
        }
        None
    }
    pub(super) fn hit_unit<'a>(&self, v: &'a HumanView, x: f64, y: f64) -> Option<&'a UnitView> {
        for u in self.display_units(v).into_iter().rev() {
            let px = self.pan.0 + f64::from(u.x) * 32. * self.zoom;
            let py = self.pan.1 + f64::from(u.y) * 16. * self.zoom;
            // Match the exact culling and sprite rectangle used by map_draw.
            if !(44.0..720.0).contains(&px) || !(150.0..640.0).contains(&py) {
                continue;
            }
            let sx = (x - (px - 32. * self.zoom)) / (64. * self.zoom);
            let sy = (y - (py - 48. * self.zoom)) / (64. * self.zoom);
            if !(0.0..1.0).contains(&sx) || !(0.0..1.0).contains(&sy) {
                continue;
            }
            let sprite = self.atlas()?.sprites.get(&format!("unit-{}", u.type_id))?;
            let col = (sx * f64::from(sprite.w)) as usize;
            let row = (sy * f64::from(sprite.h)) as usize;
            // Use source alpha, not the current animated frame: a hidden blink
            // never makes a unit disappear from pointer selection.
            if sprite.rgba[(row * sprite.w as usize + col) * 4 + 3] > 0 {
                return Some(u);
            }
        }
        None
    }
}
