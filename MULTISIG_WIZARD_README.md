# Multi-Signature Creation Wizard

A comprehensive step-by-step wizard for creating multi-signature wallets with clear guidance, validation at each step, and preview functionality. Supports various signature schemes and threshold configurations.

## Features

### 🎯 Core Functionality
- **Step-by-step wizard** with 5 comprehensive steps
- **Real-time validation** with error and warning messages
- **Multiple signature schemes**: Ed25519, Secp256k1, P256
- **Flexible threshold configuration** with visual indicators
- **Security analysis** with risk assessment and recommendations
- **Preview functionality** before wallet creation

### 📋 Wizard Steps

1. **Basic Information**
   - Wallet name and description
   - Network selection (mainnet/testnet/futurenet)
   - Basic validation

2. **Configure Signers**
   - Add/remove signers dynamically
   - Configure signer weights
   - Select signature schemes
   - Public key validation

3. **Set Threshold**
   - Visual threshold slider
   - Quick presets (conservative, standard, flexible)
   - Signer requirement analysis
   - Real-time threshold visualization

4. **Advanced Settings**
   - Time lock configuration
   - Security considerations
   - Quick time presets

5. **Preview & Create**
   - Complete configuration review
   - Security score and analysis
   - Risk assessment
   - Creation confirmation

### 🔒 Security Features

- **Public key validation** for different signature schemes
- **Threshold analysis** to prevent misconfiguration
- **Security scoring** with risk identification
- **Weight distribution analysis**
- **Time lock recommendations**

## Implementation

### File Structure
```
frontend/
├── components/
│   ├── MultiSigWizard.tsx          # Main wizard component
│   └── MultiSigWizard.test.tsx     # Test suite
├── utils/
│   └── multisig.ts                 # Utility functions and types
└── app/
    └── page.tsx                    # Integration with main app

component-library/src/components/
└── MultiSigWizard.tsx              # Reusable component version
```

### Key Components

#### MultiSigWizard Component
- **Props**: `onConfigCreate`, `initialConfig`, `className`
- **State management**: React hooks for form state
- **Validation**: Step-by-step validation with error handling
- **Navigation**: Progress indicator with step validation

#### Utility Functions
- `validateMultiSigConfig()`: Comprehensive validation
- `analyzeSecurity()`: Security risk assessment
- `calculateTotalWeight()`: Weight calculations
- `getThresholdRecommendations()`: Smart threshold suggestions

### Integration

The wizard is integrated into the main application navigation:

```tsx
// Added to main navigation
{['scanner', 'report', 'analytics', 'multisig', 'settings'].map((tab) => (
  <button onClick={() => setActiveTab(tab)}>
    {tab === 'multisig' ? 'Multi-Sig' : tab.charAt(0).toUpperCase() + tab.slice(1)}
  </button>
))}
```

## Usage Examples

### Basic Usage
```tsx
import MultiSigWizard from './components/MultiSigWizard';

function App() {
  const handleConfigCreate = (config) => {
    console.log('Creating multi-sig wallet:', config);
    // Integration with Stellar SDK for wallet creation
  };

  return (
    <MultiSigWizard 
      onConfigCreate={handleConfigCreate}
      initialConfig={{ network: 'testnet' }}
    />
  );
}
```

### Advanced Configuration
```tsx
const customConfig = {
  name: 'Treasury Wallet',
  description: 'Multi-sig for treasury operations',
  network: 'mainnet',
  timeLock: 86400, // 1 day
  signers: [
    {
      id: '1',
      name: 'Director 1',
      publicKey: 'GABC123...',
      weight: 2,
      signatureScheme: 'ed25519'
    }
  ],
  threshold: 3
};

<MultiSigWizard 
  onConfigCreate={handleCreate}
  initialConfig={customConfig}
  className="custom-wizard"
/>
```

## Validation Rules

### Signer Validation
- ✅ Valid public key format for signature scheme
- ✅ Unique signer names and public keys
- ✅ Weight between 1-100
- ✅ Maximum 20 signers

### Threshold Validation
- ✅ Threshold ≥ 1
- ✅ Threshold ≤ total weight
- ✅ Warn on extreme thresholds (100% or 1%)

### Security Analysis
- 🔍 Single signer detection
- 🔍 Weight distribution analysis
- 🔍 Threshold percentage analysis
- 🔍 Time lock recommendations

## Signature Schemes

### Ed25519 (Stellar)
- Format: `G[A-Z0-9]{55}`
- Standard for Stellar network
- 56 characters starting with 'G'

### Secp256k1
- Format: `0[2-3][0-9a-fA-F]{64}`
- Compressed public key format
- 66 hex characters

### P256
- Format: `0[2-3][0-9a-fA-F]{64}`
- Compressed public key format
- 66 hex characters

## Security Recommendations

### Threshold Settings
- **Conservative**: 80% of total weight (high security)
- **Standard**: 67% of total weight (supermajority)
- **Flexible**: 51% of total weight (simple majority)

### Time Lock Settings
- **No delay**: 0 seconds (not recommended for production)
- **Short delay**: 1 hour (balanced security)
- **Medium delay**: 1 day (high security)
- **Long delay**: 1 week (maximum security)

## Testing

The wizard includes comprehensive test coverage:

```bash
# Run tests
npm test MultiSigWizard.test.tsx

# Test coverage includes:
# - Step navigation
# - Form validation
# - Signer management
# - Threshold configuration
# - Preview generation
```

## Future Enhancements

### Planned Features
- [ ] QR code scanning for public keys
- [ ] Wallet template system
- [ ] Batch signer import
- [ ] Transaction simulation
- [ ] Integration with hardware wallets
- [ ] Multi-language support

### Integration Opportunities
- [ ] Stellar SDK integration for actual wallet creation
- [ ] Ledger integration for transaction signing
- [ ] Horizon API integration for balance checks
- [ ] Soroban contract deployment

## Browser Support

- ✅ Chrome 90+
- ✅ Firefox 88+
- ✅ Safari 14+
- ✅ Edge 90+

## Dependencies

### React Dependencies
- React 18.2+
- Next.js 14.0+

### Styling
- Tailwind CSS classes
- Responsive design
- Accessibility features

## Performance

- **Lazy loading**: Components loaded on demand
- **Optimized re-renders**: useMemo and useCallback hooks
- **Efficient validation**: Step-by-step validation
- **Minimal bundle impact**: Tree-shaking enabled

## Contributing

When contributing to the multi-signature wizard:

1. Follow the existing code structure
2. Add comprehensive tests for new features
3. Update validation rules consistently
4. Consider security implications
5. Maintain backward compatibility

## License

MIT License - see LICENSE file for details.
