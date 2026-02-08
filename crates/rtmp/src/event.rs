use crate::macros::primitive_enum;

primitive_enum! {
    #[repr(u16)]
    pub enum UserControlMessageEvent {
        StreamBegin = 0,
    }
}
