use crate::message_bus::KlineHist;

pub trait Indicator {
    type Input;
    type Output;

    fn update(&mut self, input: Self::Input) -> Self::Output;
    fn update_khist(&mut self, input: KlineHist);
}
