//! Template management system for notifications

const Handlebars = require('handlebars');
const {
  NotificationChannel,
  NotificationPriority,
  VariableType,
  NotificationTemplate,
  TemplateVariable,
  TemplateRender
} = require('./types');

class TemplateError extends Error {
  constructor(message, code) {
    super(message);
    this.name = 'TemplateError';
    this.code = code;
  }
}

/**
 * Template manager for handling notification templates
 */
class TemplateManager {
  constructor() {
    this.templates = new Map();
    this.handlebars = Handlebars.create();
    
    // Register custom helpers
    this.registerCustomHelpers();
  }

  /**
   * Register custom Handlebars helpers
   */
  registerCustomHelpers() {
    // Format date helper
    this.handlebars.registerHelper('format_date', function(date) {
      if (!date) return '';
      const d = new Date(date);
      return d.toLocaleDateString();
    });

    // Format currency helper
    this.handlebars.registerHelper('format_currency', function(amount) {
      if (typeof amount !== 'number') return '';
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: 'USD'
      }).format(amount);
    });

    // Truncate helper
    this.handlebars.registerHelper('truncate', function(text, length) {
      if (!text) return '';
      const len = parseInt(length) || 50;
      if (text.length <= len) return text;
      return text.substring(0, len) + '...';
    });

    // Conditional helper for critical alerts
    this.handlebars.registerHelper('if_critical', function(conditional, options) {
      if (conditional) {
        return options.fn(this);
      }
      return options.inverse(this);
    });
  }

  /**
   * Add a new template
   */
  addTemplate(template) {
    if (!(template instanceof NotificationTemplate)) {
      template = new NotificationTemplate(template);
    }

    // Validate template syntax
    this.validateTemplate(template);

    // Register with handlebars
    try {
      this.handlebars.compile(template.bodyTemplate);
      if (template.subjectTemplate) {
        this.handlebars.compile(template.subjectTemplate);
      }
    } catch (error) {
      throw new TemplateError(`Template compilation failed: ${error.message}`, 'COMPILATION_ERROR');
    }

    this.templates.set(template.id, template);
    return template;
  }

  /**
   * Update an existing template
   */
  updateTemplate(template) {
    if (!(template instanceof NotificationTemplate)) {
      template = new NotificationTemplate(template);
    }

    if (!this.templates.has(template.id)) {
      throw new TemplateError(`Template not found: ${template.id}`, 'NOT_FOUND');
    }

    this.validateTemplate(template);
    template.updatedAt = new Date();

    this.templates.set(template.id, template);
    return template;
  }

  /**
   * Get a template by ID
   */
  getTemplate(id) {
    return this.templates.get(id) || null;
  }

  /**
   * List all templates
   */
  listTemplates() {
    return Array.from(this.templates.values());
  }

  /**
   * Render a template with context
   */
  renderTemplate(templateId, context) {
    const template = this.getTemplate(templateId);
    if (!template) {
      throw new TemplateError(`Template not found: ${templateId}`, 'NOT_FOUND');
    }

    // Validate required variables
    this.validateContext(template, context);

    try {
      // Render body
      const bodyTemplate = this.handlebars.compile(template.bodyTemplate);
      const body = bodyTemplate(context);

      // Render subject if exists
      let subject = null;
      if (template.subjectTemplate) {
        const subjectTemplate = this.handlebars.compile(template.subjectTemplate);
        subject = subjectTemplate(context);
      }

      return new TemplateRender({
        subject,
        body,
        templateId
      });
    } catch (error) {
      throw new TemplateError(`Template rendering failed: ${error.message}`, 'RENDER_ERROR');
    }
  }

  /**
   * Delete a template
   */
  deleteTemplate(id) {
    if (!this.templates.has(id)) {
      throw new TemplateError(`Template not found: ${id}`, 'NOT_FOUND');
    }

    this.templates.delete(id);
    return true;
  }

  /**
   * Validate template syntax
   */
  validateTemplate(template) {
    if (!template.bodyTemplate || template.bodyTemplate.trim() === '') {
      throw new TemplateError('Body template cannot be empty', 'INVALID_TEMPLATE');
    }

    // Check for required variables in template
    for (const variable of template.variables) {
      if (variable.required) {
        const placeholder = `{{${variable.name}}}`;
        const inBody = template.bodyTemplate.includes(placeholder);
        const inSubject = template.subjectTemplate && template.subjectTemplate.includes(placeholder);

        if (!inBody && !inSubject) {
          throw new TemplateError(
            `Required variable '${variable.name}' not found in template`,
            'INVALID_TEMPLATE'
          );
        }
      }
    }

    return true;
  }

  /**
   * Validate context against template requirements
   */
  validateContext(template, context) {
    for (const variable of template.variables) {
      if (variable.required && !(variable.name in context)) {
        if (!variable.defaultValue) {
          throw new TemplateError(
            `Required variable '${variable.name}' is missing from context`,
            'MISSING_VARIABLE'
          );
        }
      }
    }

    return true;
  }

  /**
   * Create default templates for security scanner
   */
  createDefaultTemplates() {
    const templates = [
      // Vulnerability alert template
      new NotificationTemplate({
        id: 'vulnerability_alert',
        name: 'Vulnerability Alert',
        description: 'Template for vulnerability notifications',
        subjectTemplate: '🚨 {{severity}} Vulnerability Found in {{contract_name}}',
        bodyTemplate: `Hello {{user_name}},

A {{severity}} vulnerability has been detected in your smart contract:

Contract: {{contract_name}}
Type: {{vulnerability_type}}
Description: {{description}}
Risk Score: {{risk_score}}

{{#if critical}}
⚠️ This is a critical vulnerability that requires immediate attention!
{{/if}}

Please review the full scan report at: {{report_url}}

Best regards,
Soroban Security Scanner`,
        supportedChannels: [NotificationChannel.EMAIL, NotificationChannel.IN_APP],
        defaultPriority: NotificationPriority.HIGH,
        variables: [
          new TemplateVariable({
            name: 'user_name',
            description: 'Recipient name',
            required: true,
            variableType: VariableType.STRING
          }),
          new TemplateVariable({
            name: 'severity',
            description: 'Vulnerability severity',
            required: true,
            variableType: VariableType.STRING
          }),
          new TemplateVariable({
            name: 'contract_name',
            description: 'Name of the contract',
            required: true,
            variableType: VariableType.STRING
          }),
          new TemplateVariable({
            name: 'vulnerability_type',
            description: 'Type of vulnerability',
            required: true,
            variableType: VariableType.STRING
          }),
          new TemplateVariable({
            name: 'description',
            description: 'Vulnerability description',
            required: true,
            variableType: VariableType.STRING
          }),
          new TemplateVariable({
            name: 'risk_score',
            description: 'Risk score (0-100)',
            required: true,
            variableType: VariableType.NUMBER
          }),
          new TemplateVariable({
            name: 'critical',
            description: 'Whether this is critical',
            required: false,
            defaultValue: false,
            variableType: VariableType.BOOLEAN
          }),
          new TemplateVariable({
            name: 'report_url',
            description: 'Link to full report',
            required: true,
            variableType: VariableType.URL
          })
        ]
      }),

      // Scan completed template
      new NotificationTemplate({
        id: 'scan_completed',
        name: 'Scan Completed',
        description: 'Template for scan completion notifications',
        subjectTemplate: '✅ Security Scan Completed for {{file_path}}',
        bodyTemplate: `Hello {{user_name}},

Your security scan has completed:

File: {{file_path}}
Total Issues: {{total_issues}}
Critical: {{critical_count}}
High: {{high_count}}
Medium: {{medium_count}}

{{#if has_issues}}
⚠️ Issues were found that require your attention. Please review the detailed report.
{{else}}
✅ No security issues were found. Your contract looks secure!
{{/if}}

View full results at: {{report_url}}

Best regards,
Soroban Security Scanner`,
        supportedChannels: [NotificationChannel.EMAIL, NotificationChannel.IN_APP],
        defaultPriority: NotificationPriority.NORMAL,
        variables: [
          new TemplateVariable({
            name: 'user_name',
            description: 'Recipient name',
            required: true,
            variableType: VariableType.STRING
          }),
          new TemplateVariable({
            name: 'file_path',
            description: 'Path to scanned file',
            required: true,
            variableType: VariableType.STRING
          }),
          new TemplateVariable({
            name: 'total_issues',
            description: 'Total number of issues',
            required: true,
            variableType: VariableType.NUMBER
          }),
          new TemplateVariable({
            name: 'critical_count',
            description: 'Number of critical issues',
            required: true,
            variableType: VariableType.NUMBER
          }),
          new TemplateVariable({
            name: 'high_count',
            description: 'Number of high issues',
            required: true,
            variableType: VariableType.NUMBER
          }),
          new TemplateVariable({
            name: 'medium_count',
            description: 'Number of medium issues',
            required: true,
            variableType: VariableType.NUMBER
          }),
          new TemplateVariable({
            name: 'has_issues',
            description: 'Whether issues were found',
            required: true,
            variableType: VariableType.BOOLEAN
          }),
          new TemplateVariable({
            name: 'report_url',
            description: 'Link to full report',
            required: true,
            variableType: VariableType.URL
          })
        ]
      })
    ];

    for (const template of templates) {
      this.addTemplate(template);
    }

    return templates;
  }
}

module.exports = {
  TemplateManager,
  TemplateError
};
