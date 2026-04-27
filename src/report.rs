//! Report generation for security scan results

use crate::analysis::AnalysisResult;
use crate::ScanResult;
use colored::*;
use serde_json;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum ReportFormat {
    Console,
    Json,
    Html,
    Markdown,
}

pub struct SecurityReport {
    format: ReportFormat,
}

impl SecurityReport {
    pub fn new(format: ReportFormat) -> Self {
        Self { format }
    }

    pub fn generate(&self, analysis: &AnalysisResult, output_path: Option<&Path>) -> anyhow::Result<()> {
        // Validate analysis data
        if analysis.scan_summary.total_files_scanned == 0 {
            println!("⚠️  No files were scanned in the analysis");
        }
        
        match self.format {
            ReportFormat::Console => self.generate_console_report(analysis),
            ReportFormat::Json => self.generate_json_report(analysis, output_path),
            ReportFormat::Html => self.generate_html_report(analysis, output_path),
            ReportFormat::Markdown => self.generate_markdown_report(analysis, output_path),
        }
    }

    fn generate_console_report(&self, analysis: &AnalysisResult) -> anyhow::Result<()> {
        println!("\n{}", "🔍 STELLAR SECURITY SCAN REPORT".bold().cyan());
        println!("{}", "═".repeat(50).cyan());

        // Executive Summary
        self.print_executive_summary(analysis);

        // Risk Score
        self.print_risk_score(&analysis.risk_score);

        // Vulnerability Summary
        self.print_vulnerability_summary(&analysis.vulnerability_analysis);

        // Invariant Summary
        self.print_invariant_summary(&analysis.invariant_analysis);

        // Recommendations
        self.print_recommendations(&analysis.recommendations);

        println!("\n{}", "═".repeat(50).cyan());
        println!("Scan completed in {}ms", analysis.scan_summary.scan_duration_ms);

        Ok(())
    }

    fn print_executive_summary(&self, analysis: &AnalysisResult) {
        println!("\n{}", "📊 EXECUTIVE SUMMARY".bold().yellow());
        println!("Files Scanned: {}", analysis.scan_summary.total_files_scanned);
        println!("Files with Issues: {}", analysis.scan_summary.files_with_issues.to_string().red());
        println!("Total Vulnerabilities: {}", analysis.scan_summary.total_vulnerabilities.to_string().red());
        println!("Total Invariant Violations: {}", analysis.scan_summary.total_invariant_violations.to_string().yellow());
    }

    fn print_risk_score(&self, risk_score: &crate::analysis::RiskScore) {
        println!("\n{}", "🎯 RISK ASSESSMENT".bold().yellow());
        
        let risk_level_color = match risk_score.risk_level {
            crate::analysis::RiskLevel::Critical => "red",
            crate::analysis::RiskLevel::High => "yellow",
            crate::analysis::RiskLevel::Medium => "blue",
            crate::analysis::RiskLevel::Low => "green",
        };

        println!("Overall Risk Level: {}", 
            format!("{:?} ({:.1}/10)", risk_score.risk_level, risk_score.overall_score)
                .color(risk_level_color).bold()
        );
        println!("Security Score: {:.1}/10", risk_score.security_score);
        println!("Invariant Score: {:.1}/10", risk_score.invariant_score);
    }

    fn print_vulnerability_summary(&self, analysis: &crate::analysis::VulnerabilityAnalysis) {
        println!("\n{}", "🚨 VULNERABILITY ANALYSIS".bold().red());
        
        if analysis.most_common_vulnerabilities.is_empty() {
            println!("{}", "✅ No vulnerabilities found!".green().bold());
            return;
        }

        println!("\n{}", "Top Vulnerabilities:".bold());
        for (vuln, count) in &analysis.most_common_vulnerabilities {
            println!("  • {}: {} ({})", 
                vuln.to_string().red(), 
                count.to_string().yellow(),
                vuln.severity().as_str()
            );
        }

        if !analysis.critical_files.is_empty() {
            println!("\n{}", "Critical Files:".bold().red());
            for file in &analysis.critical_files {
                println!("  • {}", file.red());
            }
        }
    }

    fn print_invariant_summary(&self, analysis: &crate::analysis::InvariantAnalysis) {
        println!("\n{}", "🔒 INVARIANT ANALYSIS".bold().yellow());
        
        if analysis.most_violated_invariants.is_empty() {
            println!("{}", "✅ All invariants properly enforced!".green().bold());
            return;
        }

        println!("\n{}", "Most Violated Invariants:".bold());
        for (invariant, count) in &analysis.most_violated_invariants {
            println!("  • {}: {} ({})", 
                invariant.to_string().yellow(), 
                count.to_string().blue(),
                invariant.severity().as_str()
            );
        }
    }

    fn print_recommendations(&self, recommendations: &[crate::analysis::Recommendation]) {
        println!("\n{}", "💡 RECOMMENDATIONS".bold().green());
        
        if recommendations.is_empty() {
            println!("{}", "✅ No recommendations - contract appears secure!".green().bold());
            return;
        }

        for (i, rec) in recommendations.iter().enumerate() {
            let priority_color = match rec.priority {
                crate::Severity::Critical => "red",
                crate::Severity::High => "yellow",
                crate::Severity::Medium => "blue",
                crate::Severity::Low => "white",
            };

            println!("\n{}. {} [{}]", 
                i + 1,
                rec.category.bold(),
                rec.priority.as_str().color(priority_color).bold()
            );
            println!("   {}", rec.description);
            if rec.affected_files.len() <= 3 {
                for file in &rec.affected_files {
                    println!("   📁 {}", file);
                }
            } else {
                println!("   📁 {} files affected", rec.affected_files.len());
            }
            println!("   💄 {}", rec.implementation_hint.italic());
        }
    }

    fn generate_json_report(&self, analysis: &AnalysisResult, output_path: Option<&Path>) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(analysis)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON report: {}", e))?;
        
        match output_path {
            Some(path) => {
                // Validate output path
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)
                            .map_err(|e| anyhow::anyhow!("Failed to create output directory {}: {}", parent.display(), e))?;
                    }
                }
                
                fs::write(path, json)
                    .map_err(|e| anyhow::anyhow!("Failed to write JSON report to {}: {}", path.display(), e))?;
                println!("✅ JSON report saved to: {}", path.display());
            }
            None => {
                println!("{}", json);
            }
        }
        
        Ok(())
    }

    fn generate_html_report(&self, analysis: &AnalysisResult, output_path: Option<&Path>) -> anyhow::Result<()> {
        let html = self.create_html_content(analysis);
        
        match output_path {
            Some(path) => {
                // Validate output path
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)
                            .map_err(|e| anyhow::anyhow!("Failed to create output directory {}: {}", parent.display(), e))?;
                    }
                }
                
                fs::write(path, html)
                    .map_err(|e| anyhow::anyhow!("Failed to write HTML report to {}: {}", path.display(), e))?;                
                println!("✅ HTML report saved to: {}", path.display());
            }
            None => {
                println!("⚠️  HTML report content is too large for console output. Please specify an output file.");
                println!("💡 Use: --output report.html --format html");
            }
        }
        
        Ok(())
    }

    fn create_html_content(&self, analysis: &AnalysisResult) -> String {
        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Stellar Security Scan Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .header {{ text-align: center; color: #2c3e50; border-bottom: 3px solid #3498db; padding-bottom: 20px; }}
        .section {{ margin: 30px 0; }}
        .risk-score {{ text-align: center; font-size: 24px; margin: 20px 0; }}
        .critical {{ color: #e74c3c; font-weight: bold; }}
        .high {{ color: #f39c12; font-weight: bold; }}
        .medium {{ color: #3498db; font-weight: bold; }}
        .low {{ color: #27ae60; font-weight: bold; }}
        .vulnerability {{ background: #fdf2f2; border-left: 4px solid #e74c3c; padding: 15px; margin: 10px 0; }}
        .recommendation {{ background: #f0f9ff; border-left: 4px solid #3498db; padding: 15px; margin: 10px 0; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #3498db; color: white; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🔍 Stellar Security Scan Report</h1>
            <p>Generated on {}</p>
        </div>

        <div class="section">
            <h2>📊 Executive Summary</h2>
            <table>
                <tr><th>Metric</th><th>Value</th></tr>
                <tr><td>Files Scanned</td><td>{}</td></tr>
                <tr><td>Files with Issues</td><td>{}</td></tr>
                <tr><td>Total Vulnerabilities</td><td>{}</td></tr>
                <tr><td>Total Invariant Violations</td><td>{}</td></tr>
                <tr><td>Scan Duration</td><td>{}ms</td></tr>
            </table>
        </div>

        <div class="section">
            <h2>🎯 Risk Assessment</h2>
            <div class="risk-score">
                Overall Risk Level: <span class="{}">{:?} ({:.1}/10)</span><br>
                Security Score: {:.1}/10<br>
                Invariant Score: {:.1}/10
            </div>
        </div>

        <div class="section">
            <h2>🚨 Vulnerability Analysis</h2>
            {}
        </div>

        <div class="section">
            <h2>🔒 Invariant Analysis</h2>
            {}
        </div>

        <div class="section">
            <h2>💡 Recommendations</h2>
            {}
        </div>
    </div>
</body>
</html>
"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            analysis.scan_summary.total_files_scanned,
            analysis.scan_summary.files_with_issues,
            analysis.scan_summary.total_vulnerabilities,
            analysis.scan_summary.total_invariant_violations,
            analysis.scan_summary.scan_duration_ms,
            self.get_risk_class(&analysis.risk_score.risk_level),
            analysis.risk_score.risk_level,
            analysis.risk_score.overall_score,
            analysis.risk_score.security_score,
            analysis.risk_score.invariant_score,
            self.format_vulnerabilities_html(&analysis.vulnerability_analysis),
            self.format_invariants_html(&analysis.invariant_analysis),
            self.format_recommendations_html(&analysis.recommendations)
        )
    }

    fn get_risk_class(&self, risk_level: &crate::analysis::RiskLevel) -> &'static str {
        match risk_level {
            crate::analysis::RiskLevel::Critical => "critical",
            crate::analysis::RiskLevel::High => "high",
            crate::analysis::RiskLevel::Medium => "medium",
            crate::analysis::RiskLevel::Low => "low",
        }
    }

    fn format_vulnerabilities_html(&self, analysis: &crate::analysis::VulnerabilityAnalysis) -> String {
        if analysis.most_common_vulnerabilities.is_empty() {
            return "<p>✅ No vulnerabilities found!</p>".to_string();
        }

        let mut html = String::new();
        for (vuln, count) in &analysis.most_common_vulnerabilities {
            html.push_str(&format!(
                r#"<div class="vulnerability">
                    <strong>{}:</strong> {} occurrences ({})
                    <br><em>{}</em>
                </div>"#,
                vuln.to_string(),
                count,
                vuln.severity().as_str(),
                vuln.description()
            ));
        }
        html
    }

    fn format_invariants_html(&self, analysis: &crate::analysis::InvariantAnalysis) -> String {
        if analysis.most_violated_invariants.is_empty() {
            return "<p>✅ All invariants properly enforced!</p>".to_string();
        }

        let mut html = String::new();
        for (invariant, count) in &analysis.most_violated_invariants {
            html.push_str(&format!(
                r#"<div class="vulnerability">
                    <strong>{}:</strong> {} violations ({})
                    <br><em>{}</em>
                </div>"#,
                invariant.to_string(),
                count,
                invariant.severity().as_str(),
                invariant.description()
            ));
        }
        html
    }

    fn format_recommendations_html(&self, recommendations: &[crate::analysis::Recommendation]) -> String {
        if recommendations.is_empty() {
            return "<p>✅ No recommendations - contract appears secure!</p>".to_string();
        }

        let mut html = String::new();
        for (i, rec) in recommendations.iter().enumerate() {
            html.push_str(&format!(
                r#"<div class="recommendation">
                    <h3>{}. {} [{:?}]</h3>
                    <p>{}</p>
                    <p><strong>Implementation:</strong> {}</p>
                    <p><strong>Affected Files:</strong> {}</p>
                </div>"#,
                i + 1,
                rec.category,
                rec.priority,
                rec.description,
                rec.implementation_hint,
                rec.affected_files.join(", ")
            ));
        }
        html
    }

    fn generate_markdown_report(&self, analysis: &AnalysisResult, output_path: Option<&Path>) -> anyhow::Result<()> {
        let markdown = self.create_markdown_content(analysis);
        
        match output_path {
            Some(path) => {
                // Validate output path
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)
                            .map_err(|e| anyhow::anyhow!("Failed to create output directory {}: {}", parent.display(), e))?;
                    }
                }
                
                fs::write(path, markdown)
                    .map_err(|e| anyhow::anyhow!("Failed to write Markdown report to {}: {}", path.display(), e))?;
                println!("✅ Markdown report saved to: {}", path.display());
            }
            None => {
                println!("{}", markdown);
            }
        }
        
        Ok(())
    }

    fn create_markdown_content(&self, analysis: &AnalysisResult) -> String {
        format!(r#"
# 🔍 Stellar Security Scan Report

Generated on: {}

## 📊 Executive Summary

| Metric | Value |
|--------|-------|
| Files Scanned | {} |
| Files with Issues | {} |
| Total Vulnerabilities | {} |
| Total Invariant Violations | {} |
| Scan Duration | {}ms |

## 🎯 Risk Assessment

- **Overall Risk Level**: {:?} ({:.1}/10)
- **Security Score**: {:.1}/10
- **Invariant Score**: {:.1}/10

## 🚨 Vulnerability Analysis

{}

## 🔒 Invariant Analysis

{}

## 💡 Recommendations

{}
"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            analysis.scan_summary.total_files_scanned,
            analysis.scan_summary.files_with_issues,
            analysis.scan_summary.total_vulnerabilities,
            analysis.scan_summary.total_invariant_violations,
            analysis.scan_summary.scan_duration_ms,
            analysis.risk_score.risk_level,
            analysis.risk_score.overall_score,
            analysis.risk_score.security_score,
            analysis.risk_score.invariant_score,
            self.format_vulnerabilities_markdown(&analysis.vulnerability_analysis),
            self.format_invariants_markdown(&analysis.invariant_analysis),
            self.format_recommendations_markdown(&analysis.recommendations)
        )
    }

    fn format_vulnerabilities_markdown(&self, analysis: &crate::analysis::VulnerabilityAnalysis) -> String {
        if analysis.most_common_vulnerabilities.is_empty() {
            return "✅ No vulnerabilities found!".to_string();
        }

        let mut markdown = String::new();
        for (vuln, count) in &analysis.most_common_vulnerabilities {
            markdown.push_str(&format!(
                "### {} ({})\n\n**Occurrences**: {}\n\n**Description**: {}\n\n**Recommendation**: {}\n\n",
                vuln.to_string(),
                vuln.severity().as_str(),
                count,
                vuln.description(),
                vuln.recommendation()
            ));
        }
        markdown
    }

    fn format_invariants_markdown(&self, analysis: &crate::analysis::InvariantAnalysis) -> String {
        if analysis.most_violated_invariants.is_empty() {
            return "✅ All invariants properly enforced!".to_string();
        }

        let mut markdown = String::new();
        for (invariant, count) in &analysis.most_violated_invariants {
            markdown.push_str(&format!(
                "### {} ({})\n\n**Violations**: {}\n\n**Description**: {}\n\n**Recommendation**: {}\n\n",
                invariant.to_string(),
                invariant.severity().as_str(),
                count,
                invariant.description(),
                invariant.recommendation()
            ));
        }
        markdown
    }

    fn format_recommendations_markdown(&self, recommendations: &[crate::analysis::Recommendation]) -> String {
        if recommendations.is_empty() {
            return "✅ No recommendations - contract appears secure!".to_string();
        }

        let mut markdown = String::new();
        for (i, rec) in recommendations.iter().enumerate() {
            markdown.push_str(&format!(
                "### {}. {} [{:?}]\n\n**Description**: {}\n\n**Implementation**: {}\n\n**Affected Files**: {}\n\n",
                i + 1,
                rec.category,
                rec.priority,
                rec.description,
                rec.implementation_hint,
                rec.affected_files.join(", ")
            ));
        }
        markdown
    }
}
