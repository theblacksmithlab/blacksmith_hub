pub enum ProbiotRoleType {
    TestRole,
}

impl Into<&'static str> for ProbiotRoleType {
    fn into(self) -> &'static str {
        match self {
            ProbiotRoleType::TestRole => "test_role",
        }
    }
}