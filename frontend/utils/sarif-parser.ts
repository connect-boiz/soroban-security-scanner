export interface SarifLocation {
  file: string;
  line: number;
  column: number;
}

export interface SarifResult {
  id: string;
  ruleId: string;
  message: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  location: SarifLocation;
  recommendation: string;
  cweId?: string;
  title: string;
  type: string;
  fileUri?: string;
  startLine?: number;
  endLine?: number;
  startColumn?: number;
  endColumn?: number;
}

export interface SarifReport {
  version: string;
  $schema: string;
  runs: Array<{
    tool: {
      driver: {
        name: string;
        version: string;
        rules?: Array<{
          id: string;
          name: string;
          shortDescription?: {
            text: string;
          };
          fullDescription?: {
            text: string;
          };
          defaultConfiguration?: {
            level: string;
          };
          help?: {
            text: string;
          };
          properties?: {
            category?: string;
            precision?: string;
            tags?: string[];
          };
        }>;
      };
    };
    results: SarifResult[];
  }>;
}

export class SarifParser {
  static parseSarifOutput(sarifData: any): SarifResult[] {
    if (!sarifData || !sarifData.runs || !Array.isArray(sarifData.runs)) {
      return [];
    }

    const vulnerabilities: SarifResult[] = [];

    sarifData.runs.forEach((run: any) => {
      if (run.results && Array.isArray(run.results)) {
        run.results.forEach((result: any) => {
          const vulnerability = this.convertToVulnerability(result, run.tool?.driver?.rules);
          if (vulnerability) {
            vulnerabilities.push(vulnerability);
          }
        });
      }
    });

    return vulnerabilities;
  }

  private static convertToVulnerability(result: any, rules?: any[]): SarifResult | null {
    try {
      // Extract location information
      let location: SarifLocation = { file: '', line: 1, column: 1 };
      
      if (result.locations && result.locations.length > 0) {
        const physicalLocation = result.locations[0].physicalLocation;
        if (physicalLocation) {
          if (physicalLocation.artifactLocation?.uri) {
            location.file = physicalLocation.artifactLocation.uri;
          }
          
          if (physicalLocation.region) {
            location.line = physicalLocation.region.startLine || 1;
            location.column = physicalLocation.region.startColumn || 1;
          }
        }
      }

      // Determine severity
      let severity: 'critical' | 'high' | 'medium' | 'low' = 'medium';
      if (result.level) {
        switch (result.level.toLowerCase()) {
          case 'error':
            severity = 'critical';
            break;
          case 'warning':
            severity = 'high';
            break;
          case 'note':
            severity = 'medium';
            break;
          case 'info':
            severity = 'low';
            break;
        }
      }

      // Find rule information
      let ruleInfo = null;
      if (rules && result.ruleId) {
        ruleInfo = rules.find(rule => rule.id === result.ruleId);
      }

      // Extract message
      let message = '';
      if (result.message?.text) {
        message = result.message.text;
      } else if (ruleInfo?.shortDescription?.text) {
        message = ruleInfo.shortDescription.text;
      }

      // Extract title
      let title = result.ruleId || 'Unknown';
      if (ruleInfo?.name) {
        title = ruleInfo.name;
      } else if (ruleInfo?.shortDescription?.text) {
        title = ruleInfo.shortDescription.text;
      }

      // Extract type/category
      let type = 'General';
      if (ruleInfo?.properties?.category) {
        type = ruleInfo.properties.category;
      } else if (ruleInfo?.properties?.tags && ruleInfo.properties.tags.length > 0) {
        type = ruleInfo.properties.tags[0];
      }

      // Extract recommendation
      let recommendation = 'Review and fix the identified security issue.';
      if (ruleInfo?.help?.text) {
        recommendation = ruleInfo.help.text;
      }

      // Extract CWE ID
      let cweId;
      if (ruleInfo?.properties?.cwe) {
        cweId = `CWE-${ruleInfo.properties.cwe}`;
      }

      return {
        id: result.id || `${result.ruleId}-${location.file}-${location.line}`,
        ruleId: result.ruleId || 'unknown',
        message,
        severity,
        location,
        recommendation,
        cweId,
        title,
        type,
        fileUri: location.file,
        startLine: location.line,
        endLine: location.line,
        startColumn: location.column,
        endColumn: location.column
      };
    } catch (error) {
      console.error('Error converting SARIF result to vulnerability:', error);
      return null;
    }
  }

  static groupVulnerabilitiesByFile(vulnerabilities: SarifResult[]): Record<string, SarifResult[]> {
    return vulnerabilities.reduce((groups, vuln) => {
      const file = vuln.location.file || 'unknown';
      if (!groups[file]) {
        groups[file] = [];
      }
      groups[file].push(vuln);
      return groups;
    }, {} as Record<string, SarifResult[]>);
  }

  static getSeverityStats(vulnerabilities: SarifResult[]) {
    return vulnerabilities.reduce((stats, vuln) => {
      stats[vuln.severity] = (stats[vuln.severity] || 0) + 1;
      return stats;
    }, {} as Record<string, number>);
  }
}
