use http::Method;

/// Information about a registered route for conflict detection
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// The original path pattern (e.g., "/users/{id}")
    pub path: String,
    /// The HTTP methods registered for this path
    pub methods: Vec<Method>,
}

/// Error returned when a route conflict is detected
#[derive(Debug, Clone)]
pub struct RouteConflictError {
    /// The path that was being registered
    pub new_path: String,
    /// The HTTP method that conflicts
    pub method: Option<Method>,
    /// The existing path that conflicts
    pub existing_path: String,
    /// Detailed error message from the underlying router
    pub details: String,
}

impl std::fmt::Display for RouteConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "\nÔò¡ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔò«"
        )?;
        writeln!(
            f,
            "Ôöé                    ROUTE CONFLICT DETECTED                   Ôöé"
        )?;
        writeln!(
            f,
            "Ôò░ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔò»"
        )?;
        writeln!(f)?;
        writeln!(f, "  Conflicting routes:")?;
        writeln!(f, "    ÔåÆ Existing: {}", self.existing_path)?;
        writeln!(f, "    ÔåÆ New:      {}", self.new_path)?;
        writeln!(f)?;
        if let Some(ref method) = self.method {
            writeln!(f, "  HTTP Method: {}", method)?;
            writeln!(f)?;
        }
        writeln!(f, "  Details: {}", self.details)?;
        writeln!(f)?;
        writeln!(f, "  How to resolve:")?;
        writeln!(f, "    1. Use different path patterns for each route")?;
        writeln!(
            f,
            "    2. If paths must be similar, ensure parameter names differ"
        )?;
        writeln!(
            f,
            "    3. Consider using different HTTP methods if appropriate"
        )?;
        writeln!(f)?;
        writeln!(f, "  Example:")?;
        writeln!(f, "    Instead of:")?;
        writeln!(f, "      .route(\"/users/{{id}}\", get(handler1))")?;
        writeln!(f, "      .route(\"/users/{{user_id}}\", get(handler2))")?;
        writeln!(f)?;
        writeln!(f, "    Use:")?;
        writeln!(f, "      .route(\"/users/{{id}}\", get(handler1))")?;
        writeln!(f, "      .route(\"/users/{{id}}/profile\", get(handler2))")?;
        Ok(())
    }
}

impl std::error::Error for RouteConflictError {}
