#[derive(Debug, Clone, PartialEq)]
pub enum AppsSystemRoles {
    Probiot(ProbiotRoleType),
    W3A(W3ARoleType),
    RequestApp(RequestAppSystemRoleType),
    TheViperRoom(TheViperRoomRoleType),
}

impl AppsSystemRoles {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppsSystemRoles::Probiot(role) => role.as_str(),
            AppsSystemRoles::W3A(role) => role.as_str(),
            AppsSystemRoles::RequestApp(role) => role.as_str(),
            AppsSystemRoles::TheViperRoom(role) => role.as_str(),
        }
    }
}

impl AsRef<str> for AppsSystemRoles {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProbiotRoleType {
    CrapDetection,
    ClarifyRequest,
    MainProcessing,
    CrapRequestProcessing,
}

impl ProbiotRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProbiotRoleType::CrapDetection => "crap_detection",
            ProbiotRoleType::ClarifyRequest => "clarify_request",
            ProbiotRoleType::MainProcessing => "main_processing",
            ProbiotRoleType::CrapRequestProcessing => "crap_request_processing",
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
    CrapDetection,
    ClarifyRequest,
    MainProcessing,
    CrapRequestProcessing,
}

impl W3ARoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            W3ARoleType::CrapDetection => "crap_detection",
            W3ARoleType::ClarifyRequest => "clarify_request",
            W3ARoleType::MainProcessing => "main_processing",
            W3ARoleType::CrapRequestProcessing => "crap_request_processing",
        }
    }
}

impl AsRef<str> for W3ARoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestAppSystemRoleType {
    ProcessingUsersBioText,
    // ProcessingUsersRequestText,
    ReorderingResults,
}

impl RequestAppSystemRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestAppSystemRoleType::ProcessingUsersBioText => "processing_users_bio_text",
            // RequestAppSystemRoleType::ProcessingUsersRequestText => "processing_users_request_text",
            RequestAppSystemRoleType::ReorderingResults => "reordering_results",
        }
    }
}

impl AsRef<str> for RequestAppSystemRoleType {
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
    CheckUsefulness,
}

impl TheViperRoomRoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TheViperRoomRoleType::CaptionGeneration => "caption_generation",
            TheViperRoomRoleType::ExtractingNews => "extracting_news",
            TheViperRoomRoleType::SystemNicknameGeneration => "system_nickname_generation",
            TheViperRoomRoleType::CreatingPodcast => "creating_podcast",
            TheViperRoomRoleType::CheckUsefulness => "system_role_usefulness",
        }
    }
}

impl AsRef<str> for TheViperRoomRoleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
