use ractor::{
    async_trait, concurrency::tokio_primitives::JoinHandle, errors::{RactorErr, SpawnErr}, Actor,
    ActorProcessingErr, ActorRef, RpcReplyPort, call_t,
};
use std::process::ExitCode;

pub struct AccountBalance;

#[derive(Default)]
pub struct AccountBalanceArgs {
    initial_balance: i64,
    account_number: String,
}

impl AccountBalanceArgs {
    pub fn new(account_number: String) -> Self {
        Self {
            initial_balance: 0,
            account_number,
        }
    }
}

#[derive(Debug)]
pub enum AccountBalanceEventPayload {
    AmountWithdrawn { value: i64 },
    AmountDeposited { value: i64 },
    FeeApplied { value: i64 },
}

#[derive(Debug)]
pub struct AccountBalanceEvent {
    account_number: String,
    payload: AccountBalanceEventPayload,
}

#[derive(Debug)]
pub enum AccountBalanceMessage {
    ApplyEvent(AccountBalanceEvent),
    GetBalance(RpcReplyPort<i64>),
}

pub struct AccountBalanceState {
    balance: i64,
}

#[async_trait]
impl Actor for AccountBalance {
    type Msg = AccountBalanceMessage;
    type State = AccountBalanceState;
    type Arguments = AccountBalanceArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("initial balance: {}", args.initial_balance);

        Ok(Self::State { balance: args.initial_balance })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        use tracing::field;
        let tracing_span = tracing::info_span!("handle", ?message, event = field::Empty);
        let _tracing_guard = tracing_span.enter();

        match message {
            AccountBalanceMessage::ApplyEvent(event) => {
                tracing_span.record("event", field::debug(&event));
                match event.payload {
                    AccountBalanceEventPayload::AmountDeposited { value } => {
                        state.balance += value;
                    }
                    AccountBalanceEventPayload::AmountWithdrawn { value } => {
                        state.balance -= value;
                    }
                    AccountBalanceEventPayload::FeeApplied { value } => {
                        state.balance -= value;
                    }
                }
                tracing::debug!("balance after: {}", state.balance);
            }
            AccountBalanceMessage::GetBalance(reply_port) => {
                tracing::info!("sending balance: {}", state.balance);
                let _ = reply_port.send(state.balance);
            }
        }

        Ok(())
    }
}

const RPC_TIMEOUT_MS: u64 = 1000;

type AccountBalanceActorRef = ActorRef<AccountBalanceMessage>;

impl AccountBalance {
    pub async fn spawn(args: AccountBalanceArgs) -> Result<(AccountBalanceActorRef, JoinHandle<()>), SpawnErr> {
        let name = Some(Self::via(&args.account_number));
        Actor::spawn(name, Self, args).await
    }

    pub async fn apply_event(event: AccountBalanceEvent) -> Result<(), RactorErr<AccountBalanceMessage>> {
        let actor = match Self::where_is(&event.account_number) {
            Some(actor) => actor,
            None => {
                Self::spawn(AccountBalanceArgs::new(event.account_number.clone())).await?.0
            }
        };
        actor.send_message(AccountBalanceMessage::ApplyEvent(event))?;
        Ok(())
    }

    pub async fn get_balance(account_number: &str) -> Result<Option<i64>, RactorErr<AccountBalanceMessage>> {
        if let Some(actor) = Self::where_is(account_number) {
            call_t!(actor, AccountBalanceMessage::GetBalance, RPC_TIMEOUT_MS).map(Some)
        } else {
            Ok(None)
        }
    }

    fn via(account_number: &str) -> String {
        format!("{}/{}", std::any::type_name::<Self>(), account_number)
    }

    fn where_is(account_number: &str) -> Option<AccountBalanceActorRef> {
        AccountBalanceActorRef::where_is(Self::via(account_number))
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

    AccountBalance::apply_event(AccountBalanceEvent {
        account_number: "ACCOUNT1".to_string(),
        payload: AccountBalanceEventPayload::AmountDeposited { value: 100 },
    }).await?;
    AccountBalance::apply_event(AccountBalanceEvent {
        account_number: "ACCOUNT1".to_string(),
        payload: AccountBalanceEventPayload::FeeApplied { value: 5 },
    }).await?;

    for account in &["ACCOUNT1", "ACCOUNT2"] {
        let balance = AccountBalance::get_balance(account).await?;
        println!("balance of {}: {:?}", account, balance);
    }

    Ok(())
}
