#[derive(Debug, Clone, PartialEq)]
pub enum AppsSystemMessages {
    ProbiotBot(ProbiotBotMessages),
    TheViperRoom(TheViperRoomMessages),
    TheViperRoomBot(TheViperRoomBotMessages),
    RequestApp(RequestAppMessages),
    RequestAppBot(RequestAppBotMessages),
    Common(CommonMessages),
    W3ABot(W3ABotMessages),
    W3A(W3AMessages),
    GrootBot(GrootBotMessages),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrootBotMessages {
    StartMessage,
    PrivateCmdUsedInPublicChat,
    NoRightsForUseCmd,
    PublicCmdUsedInPrivateChat,
    About,
    Ask,
    AlertForViolatorChannels,
    AlertForBlackListed,
    DemoBotSystemMessage,
    DefaultScamAlert,
    AnonymousUserAlert,
    ScamDomainAlert,
    MediaRestrictionAlert,
    LLMCheckAlert,
    NoUsernameForChatAlert,
    ManualMessage,
    ResultsTempMessage,
    StartCmdReaction,
    ReportOnWhiteListedUser,
    ReportOnChatAdmin,
    ErrorDeletingMsg,
    DeletedByReport,
    ErrorProcessingMsg,
    NoUserIdWarn,
    NotCorrectReportUsage,
    GrootCmdRestrictionAlert,
}

impl GrootBotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            GrootBotMessages::StartMessage => "start_message",
            GrootBotMessages::PrivateCmdUsedInPublicChat => "private_cmd_used_in_public_chat",
            GrootBotMessages::NoRightsForUseCmd => "no_rights_for_use_cmd",
            GrootBotMessages::PublicCmdUsedInPrivateChat => "public_cmd_used_in_private_chat",
            GrootBotMessages::About => "about",
            GrootBotMessages::Ask => "ask",
            GrootBotMessages::AlertForViolatorChannels => "alert_for_violator_channels",
            GrootBotMessages::AlertForBlackListed => "alert_for_black_listed",
            GrootBotMessages::DemoBotSystemMessage => "demo_bot_system_message",
            GrootBotMessages::DefaultScamAlert => "default_scam_alert",
            GrootBotMessages::AnonymousUserAlert => "anonymous_user_alert",
            GrootBotMessages::ScamDomainAlert => "scam_domain_alert",
            GrootBotMessages::MediaRestrictionAlert => "media_restriction_alert",
            GrootBotMessages::LLMCheckAlert => "llm_check_alert",
            GrootBotMessages::NoUsernameForChatAlert => "no_username_for_chat_alert",
            GrootBotMessages::ManualMessage => "manual_message",
            GrootBotMessages::ResultsTempMessage => "results_temp_message",
            GrootBotMessages::StartCmdReaction => "start_cmd_reaction",
            GrootBotMessages::ReportOnWhiteListedUser => "report_on_white_listed_user",
            GrootBotMessages::ReportOnChatAdmin => "report_on_chat_admin",
            GrootBotMessages::ErrorDeletingMsg => "error_deleting_message",
            GrootBotMessages::DeletedByReport => "deleted_by_report",
            GrootBotMessages::ErrorProcessingMsg => "error_processing_message",
            GrootBotMessages::NoUserIdWarn => "no_user_id_warn",
            GrootBotMessages::NotCorrectReportUsage => "not_correct_report_usage",
            GrootBotMessages::GrootCmdRestrictionAlert => "groot_cmd_restriction_alert",
        }
    }
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
pub enum ProbiotBotMessages {
    StartMessage,
    PrivateChatInvitation,
    ResponseFooter,
}

impl ProbiotBotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            ProbiotBotMessages::StartMessage => "start_message",
            ProbiotBotMessages::PrivateChatInvitation => "private_chat_invitation",
            ProbiotBotMessages::ResponseFooter => "response_footer",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum W3ABotMessages {
    StartMessage,
    PrivateChatInvitation,
    ResponseFooter,
}

impl W3ABotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            W3ABotMessages::StartMessage => "start_message",
            W3ABotMessages::PrivateChatInvitation => "private_chat_invitation",
            W3ABotMessages::ResponseFooter => "response_footer",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum W3AMessages {
    W3AStudyStructure,
}

impl W3AMessages {
    pub fn as_str(&self) -> &str {
        match self {
            W3AMessages::W3AStudyStructure => "w3a_learning_structure",
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
    TestMessage1,
    TestMessage2,
}

impl RequestAppMessages {
    pub fn as_str(&self) -> &str {
        match self {
            RequestAppMessages::TestMessage1 => "test_message_1",
            RequestAppMessages::TestMessage2 => "test_message_2",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestAppBotMessages {
    TestMessage1,
    TestMessage2,
}

impl RequestAppBotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            RequestAppBotMessages::TestMessage1 => "test_message_1",
            RequestAppBotMessages::TestMessage2 => "test_message_2",
        }
    }
}
