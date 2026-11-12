//! Deterministic generated-world campaigns and replay checkpoints (#91).
//!
//! A checkpoint is a versioned setup plus the complete order transcript,
//! Rules identity and final fingerprint. Restore rebuilds the world and AI
//! players, autoplays the recorded prefix, and requires byte-identical logs.
//! This restores private AI RNG/state as well as world RNG/state, at O(turns)
//! cost. It is not a native `.SAV` or an instant state snapshot.
use crate::ai::{AiV1, BuiltInAi};
use crate::newgame::NewGameConfig;
use crate::player::Game;
use tot_kit::Rules;

/// Upper bound on rounds a campaign may run in total.
pub const MAX_ROUNDS: u32 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    TurnLimit,
    OneCivilizationRemaining(u8),
    NoCivilizationsRemaining,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiPolicy {
    AiV1,
    BuiltIn1,
}

impl AiPolicy {
    pub fn id(self) -> &'static str {
        match self {
            Self::AiV1 => "ai-v1",
            Self::BuiltIn1 => BuiltInAi::POLICY,
        }
    }
    pub fn parse(id: &str) -> Result<Self, String> {
        match id {
            "ai-v1" => Ok(Self::AiV1),
            "builtin-1" => Ok(Self::BuiltIn1),
            _ => Err(format!("unsupported campaign AI policy {id}")),
        }
    }
}

pub struct Campaign {
    pub game: Game,
    config: NewGameConfig,
    rounds: u32,
    policy: AiPolicy,
}

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

impl Campaign {
    pub fn new(config: NewGameConfig, rules: Rules) -> Result<Self, String> {
        Self::with_policy(config, rules, AiPolicy::AiV1)
    }

    pub fn with_policy(
        config: NewGameConfig,
        rules: Rules,
        policy: AiPolicy,
    ) -> Result<Self, String> {
        let mut game = Game::generated(config, rules)?;
        for civ in 1..=config.civilizations {
            game.players[civ as usize] = Some(match policy {
                AiPolicy::AiV1 => {
                    Box::new(AiV1::new(&game.rules, u64::from(config.gameplay_seed), civ))
                }
                AiPolicy::BuiltIn1 => Box::new(BuiltInAi::new(
                    &game.rules,
                    u64::from(config.gameplay_seed),
                    civ,
                )?),
            });
        }
        Ok(Self {
            game,
            config,
            rounds: 0,
            policy,
        })
    }

    pub fn policy(&self) -> AiPolicy {
        self.policy
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
        let (version, policy_line) = match self.policy {
            AiPolicy::AiV1 => (1, String::new()),
            _ => (2, format!("policy {}\n", self.policy.id())),
        };
        format!(
            "totengine-campaign {version}\nengine {}\n{policy_line}setup {}\nrounds {}\nfingerprint {:016x}\nlog {} {:016x}\n{}",
            env!("CARGO_PKG_VERSION"),
            self.config.descriptor(),
            self.rounds,
            self.game.world.checksum(),
            log.len(),
            fingerprint(log.as_bytes()),
            log
        )
    }

    pub fn restore(rules: Rules, checkpoint: &str) -> Result<Self, String> {
        let extra = usize::from(checkpoint.starts_with("totengine-campaign 2\n"));
        let fields: Vec<_> = checkpoint.splitn(7 + extra, '\n').collect();
        if fields.len() != 7 + extra
            || fields[0] != format!("totengine-campaign {}", 1 + extra)
            || fields[1] != format!("engine {}", env!("CARGO_PKG_VERSION"))
        {
            return Err("unsupported or truncated campaign checkpoint".into());
        }
        let policy = if extra == 0 {
            AiPolicy::AiV1
        } else {
            AiPolicy::parse(
                fields[2]
                    .strip_prefix("policy ")
                    .ok_or("missing AI policy")?,
            )?
        };
        if extra != usize::from(policy != AiPolicy::AiV1) {
            return Err("campaign policy and format version differ".into());
        }
        let config = NewGameConfig::from_descriptor(
            fields[2 + extra]
                .strip_prefix("setup ")
                .ok_or("missing setup")?,
        )?;
        let rounds: u32 = fields[3 + extra]
            .strip_prefix("rounds ")
            .and_then(|s| s.parse().ok())
            .ok_or("bad round count")?;
        if rounds > MAX_ROUNDS {
            return Err("checkpoint round limit exceeded".into());
        }
        let want = fields[4 + extra]
            .strip_prefix("fingerprint ")
            .ok_or("missing fingerprint")?;
        let (length, checksum) = fields[5 + extra]
            .strip_prefix("log ")
            .and_then(|s| s.split_once(' '))
            .ok_or("bad log identity")?;
        if length.parse::<usize>().ok() != Some(fields[6 + extra].len())
            || checksum != format!("{:016x}", fingerprint(fields[6 + extra].as_bytes()))
        {
            return Err("checkpoint transcript length or fingerprint differs".into());
        }
        let mut campaign = Self::with_policy(config, rules, policy)?;
        // Verify Rules/setup before potentially expensive prefix playback.
        if !fields[6 + extra].starts_with(campaign.game.log()) {
            return Err("checkpoint Rules, setup or initial state differ".into());
        }
        campaign.advance(rounds)?;
        if campaign.rounds != rounds
            || campaign.game.log() != fields[6 + extra]
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
