# Soroban Security Scanner - Bounty Marketplace Frontend

A modern web application for managing security bounties on the Soroban (Stellar) smart contract platform. This frontend provides a comprehensive bounty marketplace where organizations can post security audits and researchers can submit findings to earn XLM rewards.

## 🚀 Features

### Core Functionality
- **🎯 Bounty Board**: Browse and filter security bounties by reward amount and difficulty
- **🔒 Secure Report Submission**: Encrypt findings with owner's public key for privacy
- **⏰ Countdown Timers**: Track deadlines for active audits and First-to-Find bonuses
- **💰 Stellar Wallet Integration**: Connect Freighter wallet for XLM reward deposits
- **🏆 Leaderboard**: Rankings of security researchers based on verified findings
- **⚖️ Dispute System**: Decentralized review process for contested decisions
- **🔔 Real-time Notifications**: Instant alerts for new bounties and status updates

### Security Features
- AES encryption of sensitive findings
- Multi-signature approval system
- Role-based permissions
- Secure wallet integration
- Audit trail for all actions

### User Experience
- Responsive design for all devices
- Modern UI with Tailwind CSS
- Real-time updates via WebSocket
- Intuitive navigation and search
- Comprehensive filtering options

## 🛠 Technology Stack

- **Frontend Framework**: Next.js 14 with App Router
- **UI Library**: React 18 with TypeScript
- **Styling**: Tailwind CSS with custom components
- **State Management**: Zustand
- **Icons**: Lucide React
- **Notifications**: React Hot Toast
- **Encryption**: CryptoJS
- **Blockchain**: Stellar SDK + Freighter API
- **Real-time**: WebSocket connections
- **HTTP Client**: Axios

## 📦 Installation

1. **Clone the repository**
```bash
git clone https://github.com/Ardecrownn/soroban-security-scanner.git
cd soroban-security-scanner/frontend
```

2. **Install dependencies**
```bash
npm install
```

3. **Set up environment variables**
```bash
cp .env.example .env.local
```

Configure your environment variables in `.env.local`:
```env
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_CONTRACT_ADDRESS=your_contract_address
NEXT_PUBLIC_API_URL=http://localhost:3001
NEXT_PUBLIC_WS_URL=ws://localhost:3001/ws
```

4. **Run the development server**
```bash
npm run dev
```

5. **Open your browser**
Navigate to [http://localhost:3000](http://localhost:3000)

## 🏗️ Project Structure

```
frontend/
├── app/                    # Next.js App Router
│   ├── page.tsx           # Main application page
│   └── layout.tsx         # Root layout
├── components/            # React components
│   ├── BountyBoard.tsx   # Bounty listing and filters
│   ├── ReportSubmission.tsx # Secure report form
│   ├── Leaderboard.tsx    # Researcher rankings
│   ├── WalletConnect.tsx  # Stellar wallet integration
│   ├── Dispute.tsx        # Dispute management
│   ├── Notifications.tsx   # Real-time notifications
│   └── CountdownTimer.tsx # Deadline tracking
├── services/              # Business logic
│   ├── stellarWallet.ts   # Stellar wallet service
│   └── notificationService.ts # Notification system
├── store/                 # State management
│   └── bountyStore.ts     # Zustand store
├── types/                 # TypeScript definitions
│   └── bounty.ts          # Data models
├── utils/                 # Utility functions
│   └── encryption.ts      # Encryption utilities
├── styles/                # CSS and styling
│   └── globals.css       # Global styles
└── public/                # Static assets
```

## 🔧 Configuration

### Stellar Network Setup
- **Testnet**: Default for development
- **Mainnet**: For production deployment
- **Contract Address**: Your deployed bounty marketplace contract
- **Freighter**: Required for wallet connectivity

### Environment Variables
- `NEXT_PUBLIC_STELLAR_NETWORK`: `testnet` or `mainnet`
- `NEXT_PUBLIC_CONTRACT_ADDRESS`: Soroban contract address
- `NEXT_PUBLIC_API_URL`: Backend API endpoint
- `NEXT_PUBLIC_WS_URL`: WebSocket endpoint for real-time updates

## 🎯 Usage Guide

### For Organizations
1. **Connect Wallet**: Connect your Freighter wallet
2. **Post Bounty**: Create new security audits with XLM rewards
3. **Review Submissions**: Evaluate researcher findings
4. **Approve Rewards**: Distribute XLM to successful researchers

### For Security Researchers
1. **Browse Bounties**: Filter by reward, difficulty, and deadline
2. **Submit Findings**: Securely report vulnerabilities
3. **Track Progress**: Monitor submission status
4. **Claim Rewards**: Receive XLM for verified findings
5. **View Rankings**: Check your position on the leaderboard

### Dispute Resolution
1. **Raise Dispute**: Contest unfair rejections
2. **Provide Evidence**: Submit supporting documentation
3. **Decentralized Review**: Community-based resolution
4. **Final Decision**: Binding resolution process

## 🔐 Security Considerations

### Data Protection
- All findings encrypted with owner's public key
- Secure wallet integration with Freighter
- Role-based access control
- Audit logging for all actions

### Smart Contract Security
- Multi-signature approval system
- Time-locked reward distribution
- Input validation and sanitization
- Reentrancy protection

### Network Security
- HTTPS for all communications
- WebSocket encryption
- Rate limiting and DDoS protection
- Secure cookie handling

## 🚀 Deployment

### Vercel (Recommended)
```bash
npm install -g vercel
vercel --prod
```

### Docker
```bash
docker build -t soroban-bounty-frontend .
docker run -p 3000:3000 soroban-bounty-frontend
```

### Manual Build
```bash
npm run build
npm start
```

## 📊 API Integration

### Backend Requirements
- RESTful API for bounty management
- WebSocket support for real-time updates
- Stellar Soroban contract integration
- Authentication and authorization

### Key Endpoints
- `GET /api/bounties` - List available bounties
- `POST /api/bounties` - Create new bounty
- `POST /api/submissions` - Submit findings
- `POST /api/disputes` - Raise dispute
- `GET /api/leaderboard` - Get researcher rankings

## 🤝 Contributing

1. **Fork the repository**
2. **Create feature branch**: `git checkout -b feature/amazing-feature`
3. **Commit changes**: `git commit -m 'Add amazing feature'`
4. **Push branch**: `git push origin feature/amazing-feature`
5. **Open Pull Request**

### Development Guidelines
- Follow TypeScript best practices
- Use Tailwind CSS for styling
- Write comprehensive tests
- Document new features
- Follow semantic versioning

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Stellar Development Foundation** - Soroban smart contract platform
- **Freighter** - Stellar wallet integration
- **Next.js Team** - React framework
- **Tailwind CSS** - Utility-first CSS framework

## 📞 Support

- **Documentation**: [Project Wiki](https://github.com/Ardecrownn/soroban-security-scanner/wiki)
- **Issues**: [GitHub Issues](https://github.com/Ardecrownn/soroban-security-scanner/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Ardecrownn/soroban-security-scanner/discussions)
- **Email**: support@soroban-security-scanner.com

---

Built with ❤️ for the Stellar ecosystem
