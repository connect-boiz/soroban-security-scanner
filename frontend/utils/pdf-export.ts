import jsPDF from 'jspdf';
import html2canvas from 'html2canvas';
import { SarifResult } from './sarif-parser';

export interface VulnerabilityReportData {
  scanId: string;
  scanDate: string;
  vulnerabilities: SarifResult[];
  metrics: {
    total: number;
    critical: number;
    high: number;
    medium: number;
    low: number;
  };
  acknowledged: string[];
  falsePositives: string[];
}

export class PDFExporter {
  private doc: jsPDF;
  private pageHeight: number;
  private pageWidth: number;
  private currentY: number;
  private margin: number = 20;

  constructor() {
    this.doc = new jsPDF('p', 'mm', 'a4');
    this.pageHeight = this.doc.internal.pageSize.height;
    this.pageWidth = this.doc.internal.pageSize.width;
    this.currentY = this.margin;
  }

  async generateVulnerabilityReport(data: VulnerabilityReportData): Promise<void> {
    // Add title page
    this.addTitlePage(data);
    
    // Add summary
    this.addSummary(data);
    
    // Add detailed vulnerabilities
    this.addVulnerabilityDetails(data);
    
    // Add appendix
    this.addAppendix(data);
  }

  private addTitlePage(data: VulnerabilityReportData): void {
    // Title
    this.doc.setFontSize(24);
    this.doc.setFont('helvetica', 'bold');
    this.doc.text('Vulnerability Report', this.pageWidth / 2, 60, { align: 'center' });
    
    // Subtitle
    this.doc.setFontSize(16);
    this.doc.setFont('helvetica', 'normal');
    this.doc.text('Soroban Security Scanner', this.pageWidth / 2, 75, { align: 'center' });
    
    // Report details
    this.doc.setFontSize(12);
    const detailsY = 120;
    this.doc.text(`Scan ID: ${data.scanId}`, this.margin, detailsY);
    this.doc.text(`Date: ${data.scanDate}`, this.margin, detailsY + 10);
    this.doc.text(`Total Vulnerabilities: ${data.metrics.total}`, this.margin, detailsY + 20);
    
    // Severity breakdown
    const severityY = detailsY + 40;
    this.doc.text('Severity Breakdown:', this.margin, severityY);
    this.doc.text(`Critical: ${data.metrics.critical}`, this.margin + 20, severityY + 10);
    this.doc.text(`High: ${data.metrics.high}`, this.margin + 20, severityY + 20);
    this.doc.text(`Medium: ${data.metrics.medium}`, this.margin + 20, severityY + 30);
    this.doc.text(`Low: ${data.metrics.low}`, this.margin + 20, severityY + 40);
    
    // Footer
    this.addFooter();
    this.addNewPage();
  }

  private addSummary(data: VulnerabilityReportData): void {
    this.addHeader('Executive Summary');
    
    // Summary text
    this.doc.setFontSize(12);
    this.doc.setFont('helvetica', 'normal');
    
    const summaryText = `This security scan identified ${data.metrics.total} vulnerabilities in the analyzed Soroban smart contract. ` +
      `The findings include ${data.metrics.critical} critical, ${data.metrics.high} high, ${data.metrics.medium} medium, ` +
      `and ${data.metrics.low} low severity issues.`;
    
    const lines = this.doc.splitTextToSize(summaryText, this.pageWidth - 2 * this.margin);
    this.doc.text(lines, this.margin, this.currentY);
    this.currentY += lines.length * 7 + 10;
    
    // Risk assessment
    this.doc.setFont('helvetica', 'bold');
    this.doc.text('Risk Assessment:', this.margin, this.currentY);
    this.currentY += 10;
    
    this.doc.setFont('helvetica', 'normal');
    const riskLevel = this.calculateRiskLevel(data.metrics);
    const riskText = `Overall Risk Level: ${riskLevel}`;
    this.doc.text(riskText, this.margin, this.currentY);
    this.currentY += 10;
    
    // Recommendations summary
    this.doc.setFont('helvetica', 'bold');
    this.doc.text('Key Recommendations:', this.margin, this.currentY);
    this.currentY += 10;
    
    this.doc.setFont('helvetica', 'normal');
    const recommendations = [
      '1. Address all critical and high severity vulnerabilities immediately',
      '2. Implement proper access control mechanisms',
      '3. Add input validation and overflow checks',
      '4. Conduct thorough testing before deployment'
    ];
    
    recommendations.forEach(rec => {
      const lines = this.doc.splitTextToSize(rec, this.pageWidth - 2 * this.margin - 10);
      this.doc.text(lines, this.margin + 10, this.currentY);
      this.currentY += lines.length * 7 + 5;
    });
    
    this.addFooter();
    this.addNewPage();
  }

  private addVulnerabilityDetails(data: VulnerabilityReportData): void {
    this.addHeader('Vulnerability Details');
    
    // Group vulnerabilities by severity
    const groupedVulns = this.groupVulnerabilitiesBySeverity(data.vulnerabilities);
    
    const severities: Array<'critical' | 'high' | 'medium' | 'low'> = ['critical', 'high', 'medium', 'low'];
    
    severities.forEach(severity => {
      const vulns = groupedVulns[severity];
      if (vulns && vulns.length > 0) {
        this.addSectionHeader(`${severity.toUpperCase()} Severity (${vulns.length})`);
        
        vulns.forEach((vuln, index) => {
          if (this.currentY > this.pageHeight - 60) {
            this.addNewPage();
          }
          
          this.addVulnerabilitySection(vuln, index + 1);
        });
      }
    });
  }

  private addVulnerabilitySection(vuln: SarifResult, index: number): void {
    const startY = this.currentY;
    
    // Vulnerability title
    this.doc.setFont('helvetica', 'bold');
    this.doc.setFontSize(14);
    this.doc.text(`${index}. ${vuln.title}`, this.margin, this.currentY);
    this.currentY += 8;
    
    // Severity badge
    this.doc.setFont('helvetica', 'normal');
    this.doc.setFontSize(10);
    const severityColor = this.getSeverityColor(vuln.severity);
    this.doc.setTextColor(...severityColor);
    this.doc.text(`[${vuln.severity.toUpperCase()}]`, this.margin, this.currentY);
    this.doc.setTextColor(0, 0, 0);
    this.currentY += 8;
    
    // Location
    this.doc.setFont('helvetica', 'bold');
    this.doc.setFontSize(11);
    this.doc.text('Location:', this.margin, this.currentY);
    this.doc.setFont('helvetica', 'normal');
    this.doc.text(`${vuln.location.file}:${vuln.location.line}`, this.margin + 35, this.currentY);
    this.currentY += 7;
    
    // CWE ID
    if (vuln.cweId) {
      this.doc.setFont('helvetica', 'bold');
      this.doc.text('CWE:', this.margin, this.currentY);
      this.doc.setFont('helvetica', 'normal');
      this.doc.text(vuln.cweId, this.margin + 25, this.currentY);
      this.currentY += 7;
    }
    
    // Description
    this.doc.setFont('helvetica', 'bold');
    this.doc.text('Description:', this.margin, this.currentY);
    this.currentY += 5;
    this.doc.setFont('helvetica', 'normal');
    const descLines = this.doc.splitTextToSize(vuln.message, this.pageWidth - 2 * this.margin);
    this.doc.text(descLines, this.margin, this.currentY);
    this.currentY += descLines.length * 5 + 10;
    
    // Recommendation
    this.doc.setFont('helvetica', 'bold');
    this.doc.text('Recommendation:', this.margin, this.currentY);
    this.currentY += 5;
    this.doc.setFont('helvetica', 'normal');
    const recLines = this.doc.splitTextToSize(vuln.recommendation, this.pageWidth - 2 * this.margin);
    this.doc.text(recLines, this.margin, this.currentY);
    this.currentY += recLines.length * 5 + 15;
    
    // Add border around vulnerability section
    this.doc.setDrawColor(200, 200, 200);
    this.doc.roundedRect(
      this.margin - 5, 
      startY - 5, 
      this.pageWidth - 2 * this.margin + 10, 
      this.currentY - startY + 10, 
      2, 
      2
    );
  }

  private addAppendix(data: VulnerabilityReportData): void {
    this.addHeader('Appendix');
    
    // Acknowledged vulnerabilities
    if (data.acknowledged.length > 0) {
      this.addSectionHeader('Acknowledged Vulnerabilities');
      this.doc.setFont('helvetica', 'normal');
      this.doc.setFontSize(11);
      
      data.acknowledged.forEach(vulnId => {
        this.doc.text(`• ${vulnId}`, this.margin, this.currentY);
        this.currentY += 6;
      });
      this.currentY += 10;
    }
    
    // False positives
    if (data.falsePositives.length > 0) {
      this.addSectionHeader('False Positives');
      this.doc.setFont('helvetica', 'normal');
      this.doc.setFontSize(11);
      
      data.falsePositives.forEach(vulnId => {
        this.doc.text(`• ${vulnId}`, this.margin, this.currentY);
        this.currentY += 6;
      });
      this.currentY += 10;
    }
    
    // Disclaimer
    this.addSectionHeader('Disclaimer');
    this.doc.setFont('helvetica', 'normal');
    this.doc.setFontSize(10);
    const disclaimer = 'This report was generated by the Soroban Security Scanner. While we strive for accuracy, ' +
      'automated security analysis may produce false positives or miss certain vulnerabilities. ' +
      'Manual code review and security audits are recommended for comprehensive security assessment.';
    
    const disclaimerLines = this.doc.splitTextToSize(disclaimer, this.pageWidth - 2 * this.margin);
    this.doc.text(disclaimerLines, this.margin, this.currentY);
    
    this.addFooter();
  }

  private addHeader(title: string): void {
    this.doc.setFont('helvetica', 'bold');
    this.doc.setFontSize(18);
    this.doc.text(title, this.margin, this.currentY);
    this.currentY += 15;
  }

  private addSectionHeader(title: string): void {
    if (this.currentY > this.pageHeight - 40) {
      this.addNewPage();
    }
    
    this.doc.setFont('helvetica', 'bold');
    this.doc.setFontSize(14);
    this.doc.text(title, this.margin, this.currentY);
    this.currentY += 10;
  }

  private addFooter(): void {
    const footerY = this.pageHeight - 15;
    this.doc.setFont('helvetica', 'normal');
    this.doc.setFontSize(8);
    this.doc.text(`Generated on ${new Date().toLocaleDateString()} by Soroban Security Scanner`, this.pageWidth / 2, footerY, { align: 'center' });
    this.doc.text(`Page ${this.doc.getCurrentPageInfo().pageNumber}`, this.pageWidth - this.margin, footerY, { align: 'right' });
  }

  private addNewPage(): void {
    this.doc.addPage();
    this.currentY = this.margin;
  }

  private groupVulnerabilitiesBySeverity(vulnerabilities: SarifResult[]): Record<string, SarifResult[]> {
    return vulnerabilities.reduce((groups, vuln) => {
      if (!groups[vuln.severity]) {
        groups[vuln.severity] = [];
      }
      groups[vuln.severity].push(vuln);
      return groups;
    }, {} as Record<string, SarifResult[]>);
  }

  private calculateRiskLevel(metrics: VulnerabilityReportData['metrics']): string {
    if (metrics.critical > 0) return 'CRITICAL';
    if (metrics.high > 2) return 'HIGH';
    if (metrics.high > 0 || metrics.medium > 3) return 'MEDIUM';
    return 'LOW';
  }

  private getSeverityColor(severity: string): [number, number, number] {
    switch (severity) {
      case 'critical': return [220, 38, 38]; // red-600
      case 'high': return [245, 158, 11]; // amber-600
      case 'medium': return [59, 130, 246]; // blue-600
      case 'low': return [34, 197, 94]; // green-600
      default: return [107, 114, 128]; // gray-500
    }
  }

  async exportAsBlob(): Promise<Blob> {
    return this.doc.output('blob');
  }

  async exportAsDataURL(): Promise<string> {
    return this.doc.output('dataurlstring');
  }

  save(filename: string): void {
    this.doc.save(filename);
  }
}

// Utility function to export vulnerability report
export async function exportVulnerabilityReportPDF(data: VulnerabilityReportData): Promise<void> {
  const exporter = new PDFExporter();
  await exporter.generateVulnerabilityReport(data);
  exporter.save(`vulnerability-report-${data.scanId}.pdf`);
}

// Utility function to capture and export current view
export async function exportCurrentViewAsPDF(elementId: string, filename: string): Promise<void> {
  const element = document.getElementById(elementId);
  if (!element) {
    throw new Error(`Element with id '${elementId}' not found`);
  }

  const canvas = await html2canvas(element, {
    scale: 2,
    useCORS: true,
    allowTaint: true
  });

  const imgData = canvas.toDataURL('image/png');
  const pdf = new jsPDF('p', 'mm', 'a4');
  
  const imgWidth = 210;
  const pageHeight = 295;
  const imgHeight = (canvas.height * imgWidth) / canvas.width;
  let heightLeft = imgHeight;
  let position = 0;

  pdf.addImage(imgData, 'PNG', 0, position, imgWidth, imgHeight);
  heightLeft -= pageHeight;

  while (heightLeft >= 0) {
    position = heightLeft - imgHeight;
    pdf.addPage();
    pdf.addImage(imgData, 'PNG', 0, position, imgWidth, imgHeight);
    heightLeft -= pageHeight;
  }

  pdf.save(filename);
}
