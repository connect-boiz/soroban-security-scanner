export const siteName = 'Soroban Security Scanner';

export const siteDescription =
  'Automated security analysis for Soroban smart contracts, with vulnerability scanning, reporting, and researcher workflows.';

export const siteKeywords = [
  'Soroban security scanner',
  'Stellar smart contract audit',
  'Soroban vulnerability scanner',
  'smart contract security',
  'Stellar security tooling',
  'Web3 security analysis',
];

export function getSiteUrl() {
  const explicitUrl = process.env.NEXT_PUBLIC_SITE_URL;
  const vercelUrl = process.env.VERCEL_URL ? `https://${process.env.VERCEL_URL}` : undefined;

  return (explicitUrl || vercelUrl || 'http://localhost:3000').replace(/\/$/, '');
}

export const softwareApplicationJsonLd = {
  '@context': 'https://schema.org',
  '@type': 'SoftwareApplication',
  name: siteName,
  applicationCategory: 'SecurityApplication',
  operatingSystem: 'Web',
  description: siteDescription,
  keywords: siteKeywords.join(', '),
  featureList: [
    'Soroban smart contract vulnerability scanning',
    'Security report generation',
    'Bounty and researcher workflow management',
    'Transaction and contract analysis dashboards',
  ],
  offers: {
    '@type': 'Offer',
    price: '0',
    priceCurrency: 'USD',
  },
};
