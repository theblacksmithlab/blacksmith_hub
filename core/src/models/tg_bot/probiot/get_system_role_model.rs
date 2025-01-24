pub enum ProbiotRoleType {
    CrapDetection,
    ClarifyRequest,
}

impl Into<&'static str> for ProbiotRoleType {
    fn into(self) -> &'static str {
        match self {
            ProbiotRoleType::CrapDetection => "crap_detection",
            ProbiotRoleType::ClarifyRequest => "clarify_request",
        }
    }
}