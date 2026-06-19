export interface Bounty {
  id: string;
  title: string;
  reward: string;
  rewardAmount: number;
  difficulty: string;
  firstToFind: boolean;
}

export interface BountySubmission {
  id: string;
  bountyId: string;
  researcher: string;
  findings: string;
  encryptedFindings: string;
  severity: 'Critical' | 'High' | 'Medium' | 'Low';
  submittedAt: Date;
  status: 'Pending' | 'Approved' | 'Rejected';
}
