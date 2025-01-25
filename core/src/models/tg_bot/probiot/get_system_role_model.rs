pub enum ProbiotRoleType {
    CrapDetection,
    ClarifyRequest,
    MainProcessing,
    CrapRequestProcessing,
}

impl Into<&'static str> for ProbiotRoleType {
    fn into(self) -> &'static str {
        match self {
            ProbiotRoleType::CrapDetection => "crap_detection",
            ProbiotRoleType::ClarifyRequest => "clarify_request",
            ProbiotRoleType::MainProcessing => "main_processing",
            ProbiotRoleType::CrapRequestProcessing => "crap_request_processing"
        }
    }
}
