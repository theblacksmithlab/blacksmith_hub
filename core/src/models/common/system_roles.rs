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
    QueryTypeDefinition,
    ClarifyQuery,
    MainProcessing,
    CommonCaseQueryProcessing,
    InvalidCaseQueryProcessing,
    QueryComplexityAnalysis,
    AspectGeneration,
}

impl ProbiotRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProbiotRoleType::QueryTypeDefinition => "query_type_definition",
            ProbiotRoleType::ClarifyQuery => "clarify_query",
            ProbiotRoleType::MainProcessing => "main_processing",
            ProbiotRoleType::CommonCaseQueryProcessing => "common_case_query_processing",
            ProbiotRoleType::InvalidCaseQueryProcessing => "invalid_case_query_processing",
            ProbiotRoleType::QueryComplexityAnalysis => "query_complexity_analysis",
            ProbiotRoleType::AspectGeneration => "aspect_generation",
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
    QueryTypeDefinition,
    ClarifyQuery,
    MainProcessing,
    CommonCaseQueryProcessing,
    InvalidCaseQueryProcessing,
    SupportCaseQueryProcessing,
    TTSPreProcessing,
    QueryComplexityAnalysis,
    AspectGeneration,
}

impl W3ARoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            W3ARoleType::QueryTypeDefinition => "query_type_definition_xml",
            W3ARoleType::ClarifyQuery => "clarify_query_xml",
            W3ARoleType::MainProcessing => "main_processing_xml",
            W3ARoleType::CommonCaseQueryProcessing => "common_case_query_processing_xml",
            W3ARoleType::InvalidCaseQueryProcessing => "invalid_case_query_processing_xml",
            W3ARoleType::TTSPreProcessing => "tts_pre_processing",
            W3ARoleType::QueryComplexityAnalysis => "query_complexity_analysis_xml",
            W3ARoleType::AspectGeneration => "aspect_generation_xml",
            W3ARoleType::SupportCaseQueryProcessing => "support_case_query_processing_xml",
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
    ClarifyQuery,
    QueryTypeDefinition,
    CommonCaseQueryProcessing,
    InvalidCaseQueryProcessing,
    TTSPreProcessing,
    MainProcessing,
    QueryComplexityAnalysis,
    AspectGeneration,
}

impl BlacksmithLabRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlacksmithLabRoleType::ClarifyQuery => "clarify_query",
            BlacksmithLabRoleType::QueryTypeDefinition => "query_type_definition",
            BlacksmithLabRoleType::CommonCaseQueryProcessing => "common_case_query_processing",
            BlacksmithLabRoleType::InvalidCaseQueryProcessing => "invalid_case_query_processing",
            BlacksmithLabRoleType::TTSPreProcessing => "tts_pre_processing",
            BlacksmithLabRoleType::MainProcessing => "main_processing",
            BlacksmithLabRoleType::QueryComplexityAnalysis => "query_complexity_analysis",
            BlacksmithLabRoleType::AspectGeneration => "aspect_generation",
        }
    }
}

impl AsRef<str> for BlacksmithLabRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
