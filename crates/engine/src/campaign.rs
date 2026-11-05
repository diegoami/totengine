//! Deterministic generated-world campaigns and replay checkpoints (#91).
//!
//! A checkpoint is a versioned setup plus the complete order transcript,
//! Rules identity and final fingerprint. Restore rebuilds the world and AI
//! players, autoplays the recorded prefix, and requires byte-identical logs.
//! This restores private AI RNG/state as well as world RNG/state, at O(turns)
//! cost. It is not a native `.SAV` or an instant state snapshot.
use crate::ai::AiV1;
use crate::newgame::NewGameConfig;
use crate::player::Game;
use tot_kit::Rules;

const MAX_ROUNDS: u32 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    TurnLimit,
    OneCivilizationRemaining(u8),
    NoCivilizationsRemaining,
}

pub struct Campaign {
    pub game: Game,
    config: NewGameConfig,
    rounds: u32,
}

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

impl Campaign {
    pub fn new(config: NewGameConfig, rules: Rules) -> Result<Self, String> {
        let mut game = Game::generated(config, rules)?;
        for civ in 1..=config.civilizations {
            game.players[civ as usize] = Some(Box::new(AiV1::new(
                &game.rules,
                u64::from(config.gameplay_seed),
                civ,
            )));
        }
        Ok(Self {
            game,
            config,
            rounds: 0,
        })
    }

    pub fn rounds(&self) -> u32 {
        self.rounds
    }
    pub fn config(&self) -> NewGameConfig {
        self.config
    }

    /// These outcomes describe observed state, not the original victory rules.
    pub fn outcome(&self) -> Outcome {
        match self.game.world.living_civs().as_slice() {
            [] => Outcome::NoCivilizationsRemaining,
            [civ] => Outcome::OneCivilizationRemaining(*civ),
            _ => Outcome::TurnLimit,
        }
    }

    pub fn advance(&mut self, additional_rounds: u32) -> Result<Outcome, String> {
        if additional_rounds > MAX_ROUNDS - self.rounds {
            return Err(format!("campaign supports at most {MAX_ROUNDS} rounds"));
        }
        for _ in 0..additional_rounds {
            if self.game.world.living_civs().len() < 2 {
                break;
            }
            self.game.play_turn();
            self.rounds += 1;
        }
        Ok(self.outcome())
    }

    pub fn checkpoint(&self) -> String {
        let log = self.game.log();
        format!("totengine-campaign 1\nengine {}\nsetup {}\nrounds {}\nfingerprint {:016x}\nlog {} {:016x}\n{}",
            env!("CARGO_PKG_VERSION"), self.config.descriptor(), self.rounds,
            self.game.world.checksum(), log.len(), fingerprint(log.as_bytes()), log)
    }

    pub fn restore(rules: Rules, checkpoint: &str) -> Result<Self, String> {
        let fields: Vec<_> = checkpoint.splitn(7, '\n').collect();
        if fields.len() != 7
            || fields[0] != "totengine-campaign 1"
            || fields[1] != format!("engine {}", env!("CARGO_PKG_VERSION"))
        {
            return Err("unsupported or truncated campaign checkpoint".into());
        }
        let config = NewGameConfig::from_descriptor(
            fields[2].strip_prefix("setup ").ok_or("missing setup")?,
        )?;
        let rounds: u32 = fields[3]
            .strip_prefix("rounds ")
            .and_then(|s| s.parse().ok())
            .ok_or("bad round count")?;
        if rounds > MAX_ROUNDS {
            return Err("checkpoint round limit exceeded".into());
        }
        let want = fields[4]
            .strip_prefix("fingerprint ")
            .ok_or("missing fingerprint")?;
        let (length, checksum) = fields[5]
            .strip_prefix("log ")
            .and_then(|s| s.split_once(' '))
            .ok_or("bad log identity")?;
        if length.parse::<usize>().ok() != Some(fields[6].len())
            || checksum != format!("{:016x}", fingerprint(fields[6].as_bytes()))
        {
            return Err("checkpoint transcript length or fingerprint differs".into());
        }
        let mut campaign = Self::new(config, rules)?;
        // Verify Rules/setup before potentially expensive prefix playback.
        if !fields[6].starts_with(campaign.game.log()) {
            return Err("checkpoint Rules, setup or initial state differ".into());
        }
        campaign.advance(rounds)?;
        if campaign.rounds != rounds
            || campaign.game.log() != fields[6]
            || want != format!("{:016x}", campaign.game.world.checksum())
        {
            return Err("checkpoint differs from deterministic campaign playback".into());
        }
        if campaign.checkpoint() != checkpoint {
            return Err("noncanonical campaign checkpoint".into());
        }
        Ok(campaign)
    }
}
