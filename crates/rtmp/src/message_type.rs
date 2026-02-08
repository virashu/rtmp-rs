use crate::macros::primitive_enum;

primitive_enum! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq)]
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
        Present,
    }
}
