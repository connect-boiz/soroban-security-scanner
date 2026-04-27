//! Template management system for notifications

use crate::notification_service::types::{TemplateContext, NotificationChannel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use handlebars::{Handlebars, TemplateRenderError};

/// Template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub supported_channels: Vec<NotificationChannel>,
    pub default_priority: crate::notification_service::types::NotificationPriority,
    pub variables: Vec<TemplateVariable>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: u32,
    pub active: bool,
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default_value: Option<String>,
    pub variable_type: VariableType,
}

/// Variable types for template validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Number,
    Email,
    Phone,
    Url,
    DateTime,
    Boolean,
    Custom(String),
}

/// Template manager for handling notification templates
#[derive(Debug, Clone)]
pub struct TemplateManager {
    templates: HashMap<String, NotificationTemplate>,
    handlebars: Handlebars<'static>,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new() -> Result<Self, TemplateError> {
        let mut handlebars = Handlebars::new();
        
        // Register custom helpers
        handlebars.register_helper("format_date", Box::new(format_date_helper));
        handlebars.register_helper("format_currency", Box::new(format_currency_helper));
        handlebars.register_helper("truncate", Box::new(truncate_helper));
        
        Ok(Self {
            templates: HashMap::new(),
            handlebars,
        })
    }

    /// Add a new template
    pub fn add_template(&mut self, template: NotificationTemplate) -> Result<(), TemplateError> {
        // Validate template syntax
        self.validate_template(&template)?;
        
        // Register with handlebars
        self.handlebars.register_template(&template.id, &template.body_template)?;
        
        if let Some(subject_template) = &template.subject_template {
            let subject_id = format!("{}_subject", template.id);
            self.handlebars.register_template(&subject_id, subject_template)?;
        }
        
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// Update an existing template
    pub fn update_template(&mut self, template: NotificationTemplate) -> Result<(), TemplateError> {
        if !self.templates.contains_key(&template.id) {
            return Err(TemplateError::TemplateNotFound(template.id));
        }
        
        self.validate_template(&template)?;
        
        // Re-register with handlebars
        self.handlebars.register_template(&template.id, &template.body_template)?;
        
        if let Some(subject_template) = &template.subject_template {
            let subject_id = format!("{}_subject", template.id);
            self.handlebars.register_template(&subject_id, subject_template)?;
        }
        
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> Option<&NotificationTemplate> {
        self.templates.get(id)
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<&NotificationTemplate> {
        self.templates.values().collect()
    }

    /// Render a template with context
    pub fn render_template(
        &self,
        template_id: &str,
        context: &TemplateContext,
    ) -> Result<TemplateRender, TemplateError> {
        let template = self.templates.get(template_id)
            .ok_or_else(|| TemplateError::TemplateNotFound(template_id.to_string()))?;

        // Validate required variables
        self.validate_context(template, context)?;

        // Render body
        let body = self.handlebars.render(template_id, context)
            .map_err(|e| TemplateError::RenderError(e.to_string()))?;

        // Render subject if exists
        let subject = if let Some(_) = &template.subject_template {
            let subject_id = format!("{}_subject", template_id);
            Some(self.handlebars.render(&subject_id, context)
                .map_err(|e| TemplateError::RenderError(e.to_string()))?)
        } else {
            None
        };

        Ok(TemplateRender {
            subject,
            body,
            template_id: template_id.to_string(),
        })
    }

    /// Delete a template
    pub fn delete_template(&mut self, id: &str) -> Result<(), TemplateError> {
        if !self.templates.remove(id).is_some() {
            return Err(TemplateError::TemplateNotFound(id.to_string()));
        }
        
        self.handlebars.unregister_template(id);
        let subject_id = format!("{}_subject", id);
        self.handlebars.unregister_template(&subject_id);
        
        Ok(())
    }

    /// Validate template syntax
    fn validate_template(&self, template: &NotificationTemplate) -> Result<(), TemplateError> {
        // Basic syntax validation
        if template.body_template.is_empty() {
            return Err(TemplateError::InvalidTemplate("Body template cannot be empty".to_string()));
        }

        // Check for required variables in template
        for variable in &template.variables {
            if variable.required {
                let placeholder = format!("{{{{{}}}}}", variable.name);
                if !template.body_template.contains(&placeholder) {
                    if let Some(subject) = &template.subject_template {
                        if !subject.contains(&placeholder) {
                            return Err(TemplateError::InvalidTemplate(
                                format!("Required variable '{}' not found in template", variable.name)
                            ));
                        }
                    } else {
                        return Err(TemplateError::InvalidTemplate(
                            format!("Required variable '{}' not found in template", variable.name)
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate context against template requirements
    fn validate_context(&self, template: &NotificationTemplate, context: &TemplateContext) -> Result<(), TemplateError> {
        for variable in &template.variables {
            if variable.required && !context.contains_key(&variable.name) {
                if variable.default_value.is_none() {
                    return Err(TemplateError::MissingVariable(
                        format!("Required variable '{}' is missing from context", variable.name)
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new().expect("Failed to create TemplateManager")
    }
}

/// Rendered template result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRender {
    pub subject: Option<String>,
    pub body: String,
    pub template_id: String,
}

/// Template errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    
    #[error("Invalid template: {0}")]
    InvalidTemplate(String),
    
    #[error("Render error: {0}")]
    RenderError(String),
    
    #[error("Missing variable: {0}")]
    MissingVariable(String),
    
    #[error("Handlebars error: {0}")]
    HandlebarsError(#[from] TemplateRenderError),
}

// Custom handlebars helpers
fn format_date_helper(
    h: &handlebars::Helper<'_, '_>,
    _: &handlebars::Handlebars<'_>,
    _: &handlebars::Context,
    _: &handlebars::RenderContext<'_, '_>,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0).ok_or_else(|| handlebars::RenderError::new("Missing parameter"))?;
    let date_str = param.value().as_str().ok_or_else(|| handlebars::RenderError::new("Parameter must be a string"))?;
    
    // Simple date formatting - in a real implementation, use chrono
    let formatted = format!("Date: {}", date_str);
    out.write(&formatted)?;
    Ok(())
}

fn format_currency_helper(
    h: &handlebars::Helper<'_, '_>,
    _: &handlebars::Handlebars<'_>,
    _: &handlebars::Context,
    _: &handlebars::RenderContext<'_, '_>,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param = h.param(0).ok_or_else(|| handlebars::RenderError::new("Missing parameter"))?;
    let amount = param.value().as_f64().ok_or_else(|| handlebars::RenderError::new("Parameter must be a number"))?;
    
    let formatted = format!("${:.2}", amount);
    out.write(&formatted)?;
    Ok(())
}

fn truncate_helper(
    h: &handlebars::Helper<'_, '_>,
    _: &handlebars::Handlebars<'_>,
    _: &handlebars::Context,
    _: &handlebars::RenderContext<'_, '_>,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let text_param = h.param(0).ok_or_else(|| handlebars::RenderError::new("Missing text parameter"))?;
    let length_param = h.param(1).ok_or_else(|| handlebars::RenderError::new("Missing length parameter"))?;
    
    let text = text_param.value().as_str().ok_or_else(|| handlebars::RenderError::new("Text parameter must be a string"))?;
    let length = length_param.value().as_u64().ok_or_else(|| handlebars::RenderError::new("Length parameter must be a number"))? as usize;
    
    let truncated = if text.len() > length {
        format!("{}...", &text[..length])
    } else {
        text.to_string()
    };
    
    out.write(&truncated)?;
    Ok(())
}
