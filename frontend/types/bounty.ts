export interface Bounty {
  id: string;
  title: string;
  description: string;
  rewardAmount: number;
  difficulty: 'Easy' | 'Medium' | 'Hard' | 'Critical';
  status: 'Active' | 'InReview' | 'Approved' | 'Rejected' | 'Completed' | 'Timelocked';
  creator: string;
  assignedResearcher?: string;
  createdAt: Date;
  deadline?: Date;
  contractAddress: string;
  firstToFind: boolean;
  tags: string[];
  severity?: 'Critical' | 'High' | 'Medium' | 'Low';
}

export interface Researcher {
  address: string;
  username: string;
  reputation: number;
  completedBounties: number;
  totalEarned: number;
  rank: number;
  badges: string[];
}

export interface BountySubmission {
  id: string;
  bountyId: string;
  researcher: string;
  findings: string;
  encryptedFindings: string;
  severity: 'Critical' | 'High' | 'Medium' | 'Low';
  submittedAt: Date;
  status: 'Pending' | 'UnderReview' | 'Approved' | 'Rejected';
  adminApproved?: boolean;
  ownerApproved?: boolean;
  disputeRaised?: boolean;
}

export interface FilterOptions {
  minReward: number;
  maxReward: number;
  difficulty: string[];
  status: string[];
  tags: string[];
}

export interface NotificationData {
  id: string;
  type: 'new_bounty' | 'submission_approved' | 'bounty_completed' | 'dispute_raised';
  title: string;
  message: string;
  timestamp: Date;
  read: boolean;
}
