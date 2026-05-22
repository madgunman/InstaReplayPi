//! GPIO button abstraction for Raspberry Pi appliance deployments (stub).

pub trait GpioButtonSource: Send {
    fn poll(&mut self) -> Option<GpioEvent>;
}

pub enum GpioEvent {
    Mark,
    Replay,
    ReplayLast,
    ReturnLive,
}

/// Placeholder implementation for future Pi GPIO wiring.
pub struct StubGpio;

impl GpioButtonSource for StubGpio {
    fn poll(&mut self) -> Option<GpioEvent> {
        None
    }
}
