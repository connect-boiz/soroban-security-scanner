# Coverage Heatmap Implementation

## Overview

This implementation adds a comprehensive code coverage heatmap overlay to the Soroban Security Scanner, providing visual feedback on which parts of smart contracts were exercised by the invariant fuzzer.

## Features Implemented

### ✅ WASM Execution Engine Coverage Tracking

**Location**: `invariant-fuzzer/src/executor.rs`

- **Coverage Data Collection**: Tracks line hits, branch coverage, and function execution
- **Fuzzer Input Mapping**: Associates specific fuzzer inputs with executed lines
- **Instruction Counting**: Monitors fuel consumption for coverage metrics
- **New API Endpoints**:
  - `/fuzz` - Enhanced to return coverage data
  - `/fuzzer-inputs` - Get fuzzer inputs for specific lines

### ✅ Visual Heatmap Overlay

**Location**: `frontend/components/CoverageHeatmap.tsx`

- **Color-Coded Lines**:
  - 🟢 **Green**: Fully covered lines
  - 🟡 **Yellow**: Partially covered branches  
  - 🔴 **Red**: Lines not executed
- **Hit Count Display**: Shows execution count in line gutter
- **Interactive Monaco Editor**: Click lines to see fuzzer inputs
- **Coverage Statistics**: Real-time line and branch coverage percentages

### ✅ Branch Coverage Analysis

- **Percentage Calculation**: Shows % of if/else paths tested
- **Visual Indicators**: Color codes based on branch completeness
- **Critical Path Identification**: Highlights untested conditional logic

### ✅ Click-to-Show Fuzzer Inputs

- **Line Interaction**: Click any covered line to see triggering inputs
- **Input History**: Shows all fuzzer inputs that executed that line
- **Timestamp Tracking**: When each input was generated
- **Iteration Data**: Links inputs to specific fuzzing iterations

### ✅ Low Coverage Alerts

- **Automatic Detection**: Alerts when coverage < 60%
- **Critical Function Focus**: Identifies untested business logic
- **Visual Warnings**: Red alert banners for insufficient testing
- **Recommendations**: Suggests areas needing more testing

### ✅ Coverage-Vulnerability Correlation

**Location**: `frontend/components/CoverageReportViewer.tsx`

- **Correlation Analysis**: Maps vulnerabilities to coverage status
- **Risk Assessment**: Identifies bugs in untested code
- **Summary Reports**: Exportable coverage findings
- **Trend Analysis**: Track coverage improvements over time

### ✅ IDE-like Integration

- **Monaco Editor**: Professional code editing experience
- **Seamless Navigation**: Jump between coverage and vulnerability views
- **Responsive Design**: Works on desktop and tablet interfaces
- **Export Functionality**: PDF and JSON report generation

## Architecture

### Backend (Rust)

```rust
// Coverage data structures
pub struct CoverageData {
    pub lines_hit: HashMap<u32, u32>,
    pub branches_hit: HashMap<u32, Vec<bool>>,
    pub functions_hit: HashMap<String, u32>,
    pub total_instructions: u32,
    pub executed_instructions: u32,
}

pub struct FuzzerInput {
    pub values: Vec<FuzzValue>,
    pub iteration: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### Frontend (TypeScript/React)

```typescript
// Coverage heatmap component
interface CoverageHeatmapProps {
  fileContent: string;
  coverageData: CoverageData;
  fileName: string;
  onLineClick?: (lineNumber: number, fuzzerInputs: FuzzerInput[]) => void;
}
```

## Usage

### 1. Run Fuzzer with Coverage

```bash
# Start invariant fuzzer worker
cd invariant-fuzzer
cargo run

# Send fuzzing request with coverage tracking
curl -X POST http://localhost:8081/fuzz \
  -H "Content-Type: application/json" \
  -d '{
    "wasm_base64": "...",
    "function": "mint",
    "iterations": 1000,
    "arg_count": 3
  }'
```

### 2. View Coverage Heatmap

Navigate to `/coverage-demo` to see the interactive coverage heatmap with:
- Real-time coverage visualization
- Interactive line clicking
- Fuzzer input inspection
- Coverage statistics

### 3. Integrated Report Viewer

Navigate to `/report-viewer` to see:
- Vulnerability findings
- Coverage heatmap toggle
- Correlation analysis
- Export capabilities

## API Response Format

### Fuzzing Response with Coverage

```json
{
  "success": true,
  "iterations_completed": 1000,
  "failure_input_sequence": null,
  "error_message": null,
  "coverage_data": {
    "lines_hit": {
      "15": 8,
      "25": 5,
      "35": 0
    },
    "branches_hit": {
      "15": [true, false],
      "25": [true, true],
      "35": [false, false]
    },
    "functions_hit": {
      "new": 10,
      "mint": 8,
      "transfer": 5,
      "get_balance": 0
    },
    "total_instructions": 2000000,
    "executed_instructions": 450000
  }
}
```

### Fuzzer Inputs for Line

```json
{
  "line_number": 15,
  "fuzzer_inputs": [
    {
      "values": [42, true, "admin"],
      "iteration": 1,
      "timestamp": "2024-03-27T01:15:30Z"
    }
  ],
  "input_count": 1
}
```

## Configuration

### Coverage Thresholds

- **Excellent Coverage**: ≥ 80%
- **Good Coverage**: 60-79%
- **Low Coverage**: < 60% (triggers alerts)

### Critical Functions

Functions marked as critical if:
- Handle financial operations
- Manage access control
- Modify contract state
- Not covered by fuzzer

## Performance Considerations

### Memory Usage
- Coverage data stored in memory during fuzzing
- Fuzzer inputs mapped to lines for quick lookup
- Automatic cleanup after job completion

### Performance Optimizations
- Incremental coverage updates
- Efficient line-to-input mapping
- Lazy loading of fuzzer input details

## Future Enhancements

### Planned Features
- [ ] Historical coverage tracking
- [ ] Coverage trend visualization
- [ ] Automated test suggestion
- [ ] Integration with CI/CD pipelines
- [ ] Real-time coverage streaming
- [ ] Coverage-based test prioritization

### Technical Improvements
- [ ] WASM instrumentation for precise coverage
- [ ] Branch prediction analysis
- [ ] Coverage diff between runs
- [ ] Performance profiling integration

## Testing

### Unit Tests
```bash
# Test coverage data structures
cd invariant-fuzzer
cargo test coverage

# Test frontend components
cd frontend
npm test -- --testPathPattern=coverage
```

### Integration Tests
```bash
# Test full coverage workflow
./scripts/test-coverage-integration.sh
```

## Troubleshooting

### Common Issues

1. **Missing Coverage Data**
   - Ensure WASM module includes debug symbols
   - Check fuzzer is running with coverage enabled

2. **Performance Issues**
   - Reduce fuzzing iterations for large contracts
   - Enable coverage streaming for long runs

3. **Display Issues**
   - Clear browser cache
   - Ensure Monaco Editor loads correctly

### Debug Mode

Enable debug logging:
```bash
RUST_LOG=debug cargo run
```

## Contributing

When adding coverage features:
1. Update both backend and frontend components
2. Add comprehensive tests
3. Update this documentation
4. Consider performance implications

## License

This coverage heatmap implementation follows the same license as the main Soroban Security Scanner project.
