use std::process::ExitCode;
use thiserror::Error;
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};

pub struct Calculator;

#[derive(Debug, Clone)]
pub enum CalculatorCommand {
    Add { value: i64 },
    Sub { value: i64 },
    Mul { value: i64 },
    Div { value: i64 },
}

#[derive(Debug)]
pub struct CalculatorState {
    value: i64,
}

#[derive(Debug)]
enum CalculatorEvent {
    DidAdd { value: i64 },
    DidSub { value: i64 },
    DidMul { value: i64 },
    DidDiv { value: i64 },
}

#[derive(Error, Debug)]
pub enum CalculatorError {
    #[error("Division by zero")]
    DivisionByZero,
}

#[async_trait]
impl Actor for Calculator {
    type Msg = CalculatorCommand;
    type State = CalculatorState;
    type Arguments = i64;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(CalculatorState { value: args })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        command: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        use tracing::field;
        let tracing_span = tracing::info_span!("handle", ?command, event = field::Empty);
        let _tracing_guard = tracing_span.enter();

        match state.handle_command(command) {
            Ok(event) => {
                tracing_span.record("event", field::debug(&event));
                state.handle_event(event)
            },
            Err(err) => {
                tracing::error!("failed to handle command: {}", err);
                Ok(())
            }
        }
    }
}

impl CalculatorState {
    fn handle_command(&self, command: CalculatorCommand) -> Result<CalculatorEvent, CalculatorError> {
        match command {
            CalculatorCommand::Add { value } => Ok(CalculatorEvent::DidAdd { value }),
            CalculatorCommand::Sub { value } => Ok(CalculatorEvent::DidSub { value }),
            CalculatorCommand::Mul { value } => Ok(CalculatorEvent::DidMul { value }),
            CalculatorCommand::Div { value: 0 } => Err(CalculatorError::DivisionByZero),
            CalculatorCommand::Div { value } => Ok(CalculatorEvent::DidDiv { value }),
        }
    }

    fn handle_event(&mut self, event: CalculatorEvent) -> Result<(), ActorProcessingErr> {
        match event {
            CalculatorEvent::DidAdd { value } => self.value += value,
            CalculatorEvent::DidSub { value } => self.value -= value,
            CalculatorEvent::DidMul { value } => self.value *= value,
            CalculatorEvent::DidDiv { value } => self.value /= value,
        }
        tracing::debug!("state: {:?}", self);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match inner().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {}", err);
            ExitCode::from(1)
        }
    }
}

async fn inner() -> anyhow::Result<()> {
    local_logging::init()?;
    let (actor, handle) = Actor::spawn(None, Calculator, 0).await?;
    actor.send_message(CalculatorCommand::Add { value: 8 })?;
    actor.send_message(CalculatorCommand::Div { value: 2 })?;
    actor.send_message(CalculatorCommand::Div { value: 0 })?;
    actor.send_message(CalculatorCommand::Mul { value: 3 })?;
    actor.send_message(CalculatorCommand::Sub { value: 9 })?;
    actor.drain()?;
    handle.await?;
    Ok(())
}
