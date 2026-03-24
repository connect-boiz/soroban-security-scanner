/** @type {import('next').NextConfig} */
const nextConfig = {
  experimental: {
    appDir: true,
  },
  env: {
    STELLAR_NETWORK: process.env.STELLAR_NETWORK || 'testnet',
    CONTRACT_ADDRESS: process.env.CONTRACT_ADDRESS || '',
    API_URL: process.env.API_URL || 'http://localhost:3001',
  },
}

module.exports = nextConfig
