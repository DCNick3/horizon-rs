#[repr(u16)]
pub enum CommandType {
    Invalid = 0,
    LegacyRequest = 1,
    Close = 2,
    LegacyControl = 3,
    Request = 4,
    Control = 5,
    RequestWithContext = 6,
    ControlWithContext = 7,
}

pub struct DomainRequest {
    //
}

pub struct Request<'a, T> {
    ty: CommandType,
    domain: Option<DomainRequest>,
    data: &'a T,
}
