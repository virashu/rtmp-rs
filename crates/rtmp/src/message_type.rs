macro_rules! auto_try_from_u8 {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $vname:ident $(= $val:expr)?
            ),*
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $vname $(= $val)?
            ),*
        }

        impl std::convert::TryFrom<u8> for $name {
            type Error = anyhow::Error;

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $(
                        x if x == $name::$vname as u8 => Ok($name::$vname),
                    )*
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
        //
        // Protocol Control Messages
        //
        SetChunkSize = 0x01,
        AbortMessage = 0x02,
        Acknowledgement = 0x03,
        WindowAcknowledgementSize = 0x05,
        SetPeerBandwidth = 0x06,
        VirtualControl = 0x07,
        AudioPacket = 0x08,
        VideoPacket = 0x09,

        //
        // User Control Messages
        //
        UserControlMessage = 0x04,

        //
        // RTMP Command Messages
        //
        DataExt = 0x0F,
        ContainerExt,
        CommandExt,
        Data,
        Container,
        Command,
        Udp,
        Aggregate,
        Present
    }
}
