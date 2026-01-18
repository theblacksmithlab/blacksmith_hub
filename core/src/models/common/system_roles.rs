#[derive(Debug, Clone, PartialEq)]
pub enum AppsSystemRoles {
    Probiot(ProbiotRoleType),
    W3A(W3ARoleType),
    TheViperRoom(TheViperRoomRoleType),
    Groot(GrootRoleType),
    BlacksmithLab(BlacksmithLabRoleType),
    UniframeStudio(UniframeStudioRoleType),
    AgentDavon(AgentDavonRoleType),
}

impl AppsSystemRoles {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppsSystemRoles::Probiot(role) => role.as_str(),
            AppsSystemRoles::W3A(role) => role.as_str(),
            AppsSystemRoles::TheViperRoom(role) => role.as_str(),
            AppsSystemRoles::Groot(role) => role.as_str(),
            AppsSystemRoles::BlacksmithLab(role) => role.as_str(),
            AppsSystemRoles::UniframeStudio(role) => role.as_str(),
            AppsSystemRoles::AgentDavon(role) => role.as_str(),
        }
    }
}

impl AsRef<str> for AppsSystemRoles {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrootRoleType {
    MessageCheck,
}

impl GrootRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GrootRoleType::MessageCheck => "message_check",
        }
    }
}

impl AsRef<str> for GrootRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentDavonRoleType {
    MessageCheck,
}

impl AgentDavonRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentDavonRoleType::MessageCheck => "message_check",
        }
    }
}

impl AsRef<str> for AgentDavonRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UniframeStudioRoleType {
    ValidateKeywords,
}

impl UniframeStudioRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            UniframeStudioRoleType::ValidateKeywords => "validate_keywords",
        }
    }
}

impl AsRef<str> for UniframeStudioRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProbiotRoleType {
    RequestTypeDetection,
    ClarifyRequest,
    MainProcessing,
    CommonCaseRequestProcessing,
    InvalidCaseRequestProcessing,
}

impl ProbiotRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProbiotRoleType::RequestTypeDetection => "request_type_detection",
            ProbiotRoleType::ClarifyRequest => "clarify_request",
            ProbiotRoleType::MainProcessing => "main_processing",
            ProbiotRoleType::CommonCaseRequestProcessing => "common_case_request_processing",
            ProbiotRoleType::InvalidCaseRequestProcessing => "invalid_case_request_processing",
        }
    }
}

impl AsRef<str> for ProbiotRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum W3ARoleType {
    RequestTypeDetection,
    ClarifyRequest,
    MainProcessing,
    CommonCaseRequestProcessing,
    InvalidCaseRequestProcessing,
    Recommendation,
    TTSPreProcessing,
    QueryComplexityAnalysis,
    AspectGeneration,
}

impl W3ARoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            W3ARoleType::RequestTypeDetection => "request_type_detection",
            W3ARoleType::ClarifyRequest => "clarify_request",
            W3ARoleType::MainProcessing => "main_processing",
            W3ARoleType::CommonCaseRequestProcessing => "common_case_request_processing",
            W3ARoleType::InvalidCaseRequestProcessing => "invalid_case_request_processing",
            W3ARoleType::Recommendation => "recommendation",
            W3ARoleType::TTSPreProcessing => "tts_pre_processing",
            W3ARoleType::QueryComplexityAnalysis => "query_complexity_analysis",
            W3ARoleType::AspectGeneration => "aspect_generation",
        }
    }
}

impl AsRef<str> for W3ARoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TheViperRoomRoleType {
    CaptionGeneration,
    ExtractingNews,
    SystemNicknameGeneration,
    CreatingPodcast,
    CheckPublicUsefulness,
    CheckPrivateUsefulness,
}

impl TheViperRoomRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TheViperRoomRoleType::CaptionGeneration => "caption_generation",
            TheViperRoomRoleType::ExtractingNews => "extracting_news",
            TheViperRoomRoleType::SystemNicknameGeneration => "system_nickname_generation",
            TheViperRoomRoleType::CreatingPodcast => "creating_podcast_xml",
            TheViperRoomRoleType::CheckPublicUsefulness => "system_role_public_usefulness",
            TheViperRoomRoleType::CheckPrivateUsefulness => "system_role_private_usefulness",
        }
    }
}

impl AsRef<str> for TheViperRoomRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlacksmithLabRoleType {
    ClarifyRequest,
    RequestTypeDetection,
    CommonCaseRequestProcessing,
    InvalidCaseRequestProcessing,
    TTSPreProcessing,
    MainProcessing,
}

impl BlacksmithLabRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlacksmithLabRoleType::ClarifyRequest => "clarify_request",
            BlacksmithLabRoleType::RequestTypeDetection => "request_type_detection",
            BlacksmithLabRoleType::CommonCaseRequestProcessing => "common_case_request_processing",
            BlacksmithLabRoleType::InvalidCaseRequestProcessing => {
                "invalid_case_request_processing"
            }
            BlacksmithLabRoleType::TTSPreProcessing => "tts_pre_processing",
            BlacksmithLabRoleType::MainProcessing => "main_processing",
        }
    }
}

impl AsRef<str> for BlacksmithLabRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
