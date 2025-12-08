#[derive(Debug, Clone, PartialEq)]
pub enum AppsSystemMessages {
    ProbiotBot(ProbiotBotMessages),
    TheViperRoom(TheViperRoomMessages),
    TheViperRoomBot(TheViperRoomBotMessages),
    Common(CommonMessages),
    W3A(W3AMessages),
    GrootBot(GrootBotMessages),
    AgentDavon(AgentDavonMessages),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrootBotMessages {
    StartCmdUsedInPublicChat,
    PrivateCmdUsedInPublicChat,
    NoRightsForUseCmd,
    PublicCmdUsedInPrivateChat,
    About,
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
    CommonStartCmdReaction,
    ReportOnWhiteListedUser,
    ReportOnChatAdmin,
    ErrorDeletingMsg,
    DeletedByReport,
    ErrorProcessingMsg,
    NoUserIdWarn,
    NotCorrectReportUsage,
    GrootCmdRestrictionAlert,
    StartCmdUsedInPrivateChat,
    DemoModeMessage,
    NoNeedForCheckInPrivateChat,
    SubscriptionCmdUsedInPrivateChat,
    ImportantInstructions,
}

impl GrootBotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            GrootBotMessages::StartCmdUsedInPublicChat => "start_cmd_used_in_public_chat",
            GrootBotMessages::PrivateCmdUsedInPublicChat => "private_cmd_used_in_public_chat",
            GrootBotMessages::NoRightsForUseCmd => "no_rights_for_use_cmd",
            GrootBotMessages::PublicCmdUsedInPrivateChat => "public_cmd_used_in_private_chat",
            GrootBotMessages::About => "about",
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
            GrootBotMessages::CommonStartCmdReaction => "common_start_cmd_reaction",
            GrootBotMessages::ReportOnWhiteListedUser => "report_on_white_listed_user",
            GrootBotMessages::ReportOnChatAdmin => "report_on_chat_admin",
            GrootBotMessages::ErrorDeletingMsg => "error_deleting_message",
            GrootBotMessages::DeletedByReport => "deleted_by_report",
            GrootBotMessages::ErrorProcessingMsg => "error_processing_message",
            GrootBotMessages::NoUserIdWarn => "no_user_id_warn",
            GrootBotMessages::NotCorrectReportUsage => "not_correct_report_usage",
            GrootBotMessages::GrootCmdRestrictionAlert => "groot_cmd_restriction_alert",
            GrootBotMessages::StartCmdUsedInPrivateChat => "start_cmd_used_in_private_chat",
            GrootBotMessages::DemoModeMessage => "demo_mode_message",
            GrootBotMessages::NoNeedForCheckInPrivateChat => "no_need_for_check_in_private_chat",
            GrootBotMessages::SubscriptionCmdUsedInPrivateChat => {
                "subscription_cmd_used_in_private_chat"
            }
            GrootBotMessages::ImportantInstructions => "important_instructions",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentDavonMessages {
    Offer,
}

impl AgentDavonMessages {
    pub fn as_str(&self) -> &str {
        match self {
            AgentDavonMessages::Offer => "offer",
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
    PrivateCmdUsedInPublicChat,
    NoUsernameWarning,
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
            CommonMessages::PrivateCmdUsedInPublicChat => "private_cmd_used_in_public_chat",
            CommonMessages::NoUsernameWarning => "no_username_warning",
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
    PublicChatMessageCommunication,
    StartMessage,
    FAQ,
    MainMenu,
    UnexpectedMessage,
    SupportMessage,
    MainMenuMinimal,
    PleaseWaitForPublicPodcastSearch,
    ChannelAddingInstruction,
    SettingsMenuUnexpectedMessage,
    GeminiTTSInstruction,
    PleaseWaitForPersonalPodcastSearch,
    PleaseWaitForPersonalPodcastRecord,
    GrabAFreshPersonalPodcast,
    GrabAFreshPublicPodcast,
    PublicPodcaseIsNotReadyYet
}

impl TheViperRoomBotMessages {
    pub fn as_str(&self) -> &str {
        match self {
            TheViperRoomBotMessages::DonationFooter => "donation_footer",
            TheViperRoomBotMessages::WrongCmdOrNoRightsMessage => "wrong_cmd_or_no_rights_message",
            TheViperRoomBotMessages::PublicChatMessageCommunication => {
                "public_chat_message_communication"
            }
            TheViperRoomBotMessages::StartMessage => "start_message",
            TheViperRoomBotMessages::FAQ => "faq",
            TheViperRoomBotMessages::MainMenu => "main_menu_text_full",
            TheViperRoomBotMessages::UnexpectedMessage => "unexpected_message",
            TheViperRoomBotMessages::SupportMessage => "support_message",
            TheViperRoomBotMessages::MainMenuMinimal => "main_menu_text_minimal",
            TheViperRoomBotMessages::PleaseWaitForPublicPodcastSearch => {
                "please_wait_for_public_podcast_search"
            }
            TheViperRoomBotMessages::ChannelAddingInstruction => "channel_adding_instruction",
            TheViperRoomBotMessages::SettingsMenuUnexpectedMessage => {
                "settings_menu_unexpected_message"
            }
            TheViperRoomBotMessages::GeminiTTSInstruction => "gemini_tts_instruction",
            TheViperRoomBotMessages::PleaseWaitForPersonalPodcastSearch => {
                "please_wait_for_personal_podcast_search"
            }
            TheViperRoomBotMessages::PleaseWaitForPersonalPodcastRecord => {
                "please_wait_for_personal_podcast_record"
            }
            TheViperRoomBotMessages::GrabAFreshPersonalPodcast => "grab_a_fresh_personal_podcast",
            TheViperRoomBotMessages::GrabAFreshPublicPodcast => "grab_a_fresh_public_podcast",
            TheViperRoomBotMessages::PublicPodcaseIsNotReadyYet => "public_podcase_is_not_ready",
        }
    }
}
