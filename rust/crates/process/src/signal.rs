use androidpc_runtime_api::Signal;

pub struct SignalTranslator;

impl SignalTranslator {
    pub fn to_unix_signal(signal: Signal) -> i32 {
        match signal {
            Signal::Hup => 1,
            Signal::Int => 2,
            Signal::Quit => 3,
            Signal::Kill => 9,
            Signal::Term => 15,
            Signal::Usr1 => 10,
            Signal::Usr2 => 12,
        }
    }
}
