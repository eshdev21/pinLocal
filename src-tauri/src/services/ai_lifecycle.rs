use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use strum::FromRepr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, FromRepr)]
#[serde(rename_all = "snake_case")]
#[repr(i32)]
pub enum EngineStatus {
    Disabled = -1,
    #[default]
    Stopped = 0,
    Starting = 1,
    Running = 2,
    Error = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, FromRepr)]
#[serde(rename_all = "snake_case")]
#[repr(i32)]
pub enum ModelStatus {
    #[default]
    Unloaded = 0,
    Loading = 1,
    Ready = 2,
    Error = 3,
}

/// Self-contained AI lifecycle state machine.
/// Invariant: when engine leaves Running, model auto-resets to Unloaded.
pub struct AiStateMachine {
    engine: AtomicI32,
    model: AtomicI32,
    pid: AtomicU32,
}

impl AiStateMachine {
    pub fn new(initial_engine: EngineStatus) -> Self {
        Self {
            engine: AtomicI32::new(initial_engine as i32),
            model: AtomicI32::new(ModelStatus::Unloaded as i32),
            pid: AtomicU32::new(0),
        }
    }

    /// Transition engine. Uses CAS. Auto-resets model on non-Running.
    pub fn transition_engine(&self, to: EngineStatus) -> bool {
        let current_raw = self.engine.load(Ordering::SeqCst);
        let current = EngineStatus::from_repr(current_raw).unwrap_or(EngineStatus::Stopped);
        
        if current == to {
            return false;
        }

        let valid = match to {
            EngineStatus::Stopped | EngineStatus::Disabled | EngineStatus::Error => true,
            EngineStatus::Starting => matches!(
                current,
                EngineStatus::Stopped | EngineStatus::Disabled | EngineStatus::Error
            ),
            EngineStatus::Running => matches!(current, EngineStatus::Starting),
        };

        if !valid {
            log::warn!("REJECTED Engine transition: {:?} -> {:?}", current, to);
            return false;
        }

        match self.engine.compare_exchange(
            current_raw,
            to as i32,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                log::info!("AI Engine: {:?} -> {:?}", current, to);
                
                // Enforce invariant: non-running engine -> unloaded model
                if !matches!(to, EngineStatus::Running | EngineStatus::Starting) {
                    self.transition_model(ModelStatus::Unloaded);
                }
                true
            }
            Err(actual_raw) => {
                log::warn!(
                    "AI Engine CAS conflict: expected {:?}, found {:?} while transitioning to {:?}",
                    current,
                    EngineStatus::from_repr(actual_raw),
                    to
                );
                false
            }
        }
    }

    /// Transition model. Uses CAS.
    pub fn transition_model(&self, to: ModelStatus) -> bool {
        let current_raw = self.model.load(Ordering::SeqCst);
        let current = ModelStatus::from_repr(current_raw).unwrap_or(ModelStatus::Unloaded);
        
        if current == to {
            return false;
        }

        let valid = match to {
            ModelStatus::Unloaded | ModelStatus::Error => true,
            ModelStatus::Loading => matches!(current, ModelStatus::Unloaded | ModelStatus::Error),
            ModelStatus::Ready => matches!(current, ModelStatus::Loading),
        };

        if !valid {
            log::warn!("REJECTED Model transition: {:?} -> {:?}", current, to);
            return false;
        }

        match self.model.compare_exchange(
            current_raw,
            to as i32,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                log::info!("AI Model: {:?} -> {:?}", current, to);
                true
            }
            Err(actual_raw) => {
                log::warn!(
                    "AI Model CAS conflict: expected {:?}, found {:?} while transitioning to {:?}",
                    current,
                    ModelStatus::from_repr(actual_raw),
                    to
                );
                false
            }
        }
    }

    pub fn engine(&self) -> EngineStatus {
        EngineStatus::from_repr(self.engine.load(Ordering::SeqCst)).unwrap_or(EngineStatus::Stopped)
    }

    pub fn model(&self) -> ModelStatus {
        ModelStatus::from_repr(self.model.load(Ordering::SeqCst)).unwrap_or(ModelStatus::Unloaded)
    }

    pub fn pid(&self) -> u32 {
        self.pid.load(Ordering::SeqCst)
    }

    pub fn set_pid(&self, pid: u32) {
        self.pid.store(pid, Ordering::SeqCst);
    }
}
