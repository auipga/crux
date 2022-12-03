//! TODO mod docs

use crux_core::{channels::Sender, Capability, Command};
use serde::{Deserialize, Serialize};

// TODO revisit this
#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResponse(pub String);

pub struct Time<Ev> {
    sender: Sender<Command<(), Ev>>,
}

impl<Ev> Time<Ev>
where
    Ev: 'static,
{
    pub fn new(sender: Sender<Command<(), Ev>>) -> Self {
        Self { sender }
    }

    pub fn get<F>(&self, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.sender.send(Command::new((), callback))
    }
}

impl<Ef> Capability for Time<Ef> {}
