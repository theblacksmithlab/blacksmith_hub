#[derive(Debug, Clone, PartialEq)]
pub enum AppsSystemMessages {
    Probiot(ProbiotMessages),
    TheViperRoom(TheViperRoomMessages),
    TheViperRoomBot(TheViperRoomBotMessages),
    RequestApp(RequestAppMessages),
    RequestAppBot(RequestAppBotMessages),
    Common(CommonMessages),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommonMessages {
    StartMessage,
    AutoReply,
    DefaultSystemErrorMessage,
    ErrorDownloadingVoiceMessageFile,
    ErrorProcessingRequest,
    ErrorProcessingVoiceMessage,
    GlobalErrorProcessingVoiceMessage,
    InvalidRequestContent,
}

impl CommonMessages {
    pub fn as_str(&self) -> &str {
        match self {
            CommonMessages::StartMessage => "start_message",
            CommonMessages::AutoReply => "auto_reply",
            CommonMessages::DefaultSystemErrorMessage => "default_system_error_message",
            CommonMessages::ErrorDownloadingVoiceMessageFile => {
                "error_downloading_voice_message_file"
            }
            CommonMessages::ErrorProcessingRequest => "error_processing_request",
            CommonMessages::ErrorProcessingVoiceMessage => "error_processing_voice_message",
            CommonMessages::GlobalErrorProcessingVoiceMessage => {
                "global_error_processing_voice_message"
            }
            CommonMessages::InvalidRequestContent => "invalid_request_content",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProbiotMessages {
    StartMessage,
    PrivateChatInvitation,
    ResponseFooter,
    CrapRequestResponse,
}

impl ProbiotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            ProbiotMessages::StartMessage => "start_message",
            ProbiotMessages::PrivateChatInvitation => "private_chat_invitation",
            ProbiotMessages::ResponseFooter => "response_footer_contacts",
            ProbiotMessages::CrapRequestResponse => "response_for_crap_request",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TheViperRoomMessages {
    DonationFooter,
}

impl TheViperRoomMessages {
    pub fn as_str(&self) -> &str {
        match self {
            TheViperRoomMessages::DonationFooter => "donation_footer",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TheViperRoomBotMessages {
    DonationFooter,
    WrongCmdOrNoRightsMessage,
}

impl TheViperRoomBotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            TheViperRoomBotMessages::DonationFooter => "donation_footer",
            TheViperRoomBotMessages::WrongCmdOrNoRightsMessage => "wrong_cmd_or_no_rights_message",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestAppMessages {
    RequestReceived,
    ErrorMessage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestAppBotMessages {
    BotStartMessage,
    ErrorMessage,
}
