use crate::detail::ViewGroupDetail;

pub(crate) enum State{
    Detail(ViewGroupDetail),
}

impl State {
    pub fn as_app(&mut self) -> &mut dyn eframe::App {
        match self {
            State::Detail(a) => a,
        }
    }
}
