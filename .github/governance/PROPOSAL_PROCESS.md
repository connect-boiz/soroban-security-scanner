# Proposal Process Guide

## Overview

This document provides step-by-step instructions for creating, submitting, and tracking governance proposals for the Soroban Security Scanner.

## Before You Begin

### Determine Proposal Type

1. **Protocol Proposal (P-XXX)**: Core protocol changes, consensus, architecture
2. **Smart Contract Proposal (SC-XXX)**: On-chain smart contract modifications
3. **Parameter Proposal (PP-XXX)**: Protocol parameter adjustments
4. **Emergency Proposal (EM-XXX)**: Critical security patches
5. **Infrastructure Proposal (I-XXX)**: Off-chain infrastructure changes
6. **Governance Proposal (G-XXX)**: Changes to governance framework itself

### Check for Existing Proposals

Search the [proposals directory](.github/governance/proposals/) and [discussions](https://github.com/connect-boiz/soroban-security-scanner/discussions) to see if similar proposals exist.

## Step 1: Draft Proposal (Off-Chain)

### Create Proposal Document

Create a markdown file in `.github/governance/proposals/drafts/` with the naming convention:
- `P-XXX-title.md` (Protocol)
- `SC-XXX-title.md` (Smart Contract)
- `PP-XXX-title.md` (Parameter)
- `EM-XXX-title.md` (Emergency)
- `I-XXX-title.md` (Infrastructure)
- `G-XXX-title.md` (Governance)

### Proposal Template

```markdown
# Proposal: [TYPE]-XXX - [TITLE]

**Author**: [Your Name/GitHub Handle]  
**Created**: [Date]  
**Status**: Draft / Discussion / Accepted / Implemented / Rejected

## Executive Summary

[1-2 paragraphs summarizing the proposal]

## Motivation

[Why this change is needed]

## Specification

[Detailed technical specification]

## Rationale

[Why this approach vs alternatives]

## Backwards Compatibility

[Analysis of compatibility impact]

## Security Considerations

[Security impact and mitigations]

## Test Cases

[Test scenarios]

## Implementation

[Implementation plan and timeline]

## References

[Links to related discussions, issues, documentation]
```

## Step 2: Initial Review

### Form a Review Team

Contact relevant stakeholders:
- Technical Lead (@technical-lead)
- Security Lead (@security-lead)
- Protocol Council (@protocol-council)

### Conduct Pre-Review

- [ ] Share draft with review team
- [ ] Incorporate feedback
- [ ] Address concerns
- [ ] Finalize proposal

## Step 3: Formal Submission

### Submit to Discussion Forum

1. Create discussion post in GitHub Discussions
2. Use appropriate category:
   - "Protocol Proposal" for P-XXX
   - "Smart Contract Proposal" for SC-XXX
   - "Parameter Change" for PP-XXX
   - "Infrastructure" for I-XXX
   - "Governance" for G-XXX

### Discussion Period

| Proposal Type | Minimum Duration |
|---------------|------------------|
| Protocol (P) | 30 days |
| Smart Contract (SC) | 14 days |
| Parameter (PP) | 7 days |
| Emergency (EM) | 24 hours |
| Infrastructure (I) | 3 days |
| Governance (G) | 30 days |

### Discussion Guidelines

- Be constructive and respectful
- Focus on technical merits
- Address concerns raised
- Update proposal based on feedback
- Avoid off-topic discussions

## Step 4: Freeze and Prepare for Vote

### Freeze Proposal

After discussion period, proposal is frozen (no more changes).

### Prepare Supporting Materials

- [ ] Final proposal document
- [ ] Implementation plan
- [ ] Security audit report (if required)
- [ ] Test results
- [ ] Cost analysis
- [ ] Risk assessment

### Create Snapshot Vote

For proposals requiring community vote:
1. Create snapshot space at [TBD]
2. Configure voting parameters
3. Generate voting link
4. Announce vote

## Step 5: Voting Period

### Voting Duration

| Proposal Type | Duration |
|---------------|----------|
| Protocol (P) | 7 days |
| Smart Contract (SC) | 5 days |
| Parameter (PP) | 3 days |
| Emergency (EM) | 24 hours |
| Infrastructure (I) | 3 days |
| Governance (G) | 10 days |

### Monitor Vote

- Track voting progress
- Answer questions
- Address issues
- Ensure fair process

## Step 6: Implementation

### If Approved

1. Move proposal to `approved/` directory
2. Assign implementation team
3. Create implementation timeline
4. Set up timelock (if required)
5. Begin development
6. Conduct security review
7. Deploy with monitoring

### If Rejected

1. Move proposal to `rejected/` directory
2. Document reasons
3. Provide feedback for improvement
4. Allow resubmission after 90 days (major changes)

## Step 7: Post-Implementation

### Documentation

- [ ] Update technical documentation
- [ ] Update user guides
- [ ] Update API documentation
- [ ] Publish implementation report

### Review

- [ ] Conduct post-implementation review
- [ ] Assess impact
- [ ] Identify lessons learned
- [ ] Update processes if needed

## Emergency Proposal Fast Track

### Conditions for Emergency

- Critical security vulnerability
- System stability threat
- Data loss risk
- Legal/compliance requirement

### Emergency Process

1. Immediate notification to council
2. 24-hour review period
3. 90% council approval required
4. Implementation with monitoring
5. Full review within 7 days

## Proposal Tracking

### Status Lifecycle

```
Draft → Submitted → Discussion → Review → Vote → [Approved/Rejected] → Implementation → Completed
                                     ↓
                                   [Modified]
```

### Directory Structure

```
.github/governance/proposals/
├── drafts/          # Active draft proposals
├── discussion/      # Under discussion
├── approved/        # Approved proposals
├── implemented/     # Completed proposals
├── rejected/        # Rejected proposals
└── archive/         # Historical proposals
```

## Proposal Numbering

Proposals are numbered sequentially by type:
- P-001, P-002, ... (Protocol)
- SC-001, SC-002, ... (Smart Contract)
- PP-001, PP-002, ... (Parameter)
- EM-001, EM-002, ... (Emergency)
- I-001, I-002, ... (Infrastructure)
- G-001, G-002, ... (Governance)

## Contact

For proposal process questions, contact:
- Governance Committee: governance@soroban-security.org
- Technical Committee: technical@soroban-security.org
- Security Team: security@soroban-security.org

## Examples

See the `examples/` directory for:
- Approved proposal examples
- Implementation reports
- Voting results
- Post-implementation reviews