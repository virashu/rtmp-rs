macro_rules! auto_try_from_u8 {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl std::convert::TryFrom<u8> for $name {
            type Error = anyhow::Error;

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u8 => Ok($name::$vname),)*
                    _ => Err(anyhow::anyhow!("Unknown value: 0x{v:x}")),
                }
            }
        }
    }
}

auto_try_from_u8! {
    #[repr(u8)]
    #[derive(Debug, PartialEq)]
    pub enum MessageType {
        SetPacketSizeMessage = 0x01,
        Abort = 0x02,
        Acknowledge,
        ControlMessage,
        ServerBandwidth,
        ClientBandwidth,
        VirtualControl,
        AudioPacket,
        VideoPacket,

        DataExt = 0x0F,
        ContainerExt,
        CommandExt,
        Data,
        Container,
        Command,
        Udp,
        Aggregate,
        Present,
    }
}
