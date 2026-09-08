import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import {
  BalanceDisplay,
  TokenBalance,
  HistoricalData,
  ConversionRate,
} from '../components/BalanceDisplay';

const tokens: TokenBalance[] = [
  {
    symbol: 'XLM',
    name: 'Stellar Lumens',
    balance: '1250.50',
    decimals: 7,
    usdValue: 187.58,
    change24h: 2.5,
    contractAddress: 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2MLQQ4RUE5667W',
  },
  {
    symbol: 'ETH',
    name: 'Ethereum',
    balance: '0.75',
    decimals: 18,
    usdValue: 1500.0,
    change24h: -1.2,
    contractAddress: 'CA3D5KRYM6CB7OWQ6TWYRR3Z4T7VN6LARJ7K36QKUQ5Q5Y6R7H5W7Q',
  },
];

const historicalData: HistoricalData[] = [
  { timestamp: Date.now() - 86400000, balance: '1000', usdValue: 1000 },
  { timestamp: Date.now(), balance: '1500', usdValue: 1500 },
];

const conversionRates: ConversionRate[] = [
  { from: 'XLM', to: 'USD', rate: 0.15, timestamp: Date.now() },
  { from: 'ETH', to: 'USD', rate: 2000, timestamp: Date.now() },
];

describe('BalanceDisplay', () => {
  it('renders token balances and portfolio totals', async () => {
    render(
      <BalanceDisplay
        tokens={tokens}
        historicalData={historicalData}
        conversionRates={conversionRates}
        realTimeUpdates={false}
      />
    );
    expect(screen.getByText('Portfolio Balance')).toBeInTheDocument();
    expect(screen.getAllByText('XLM').length).toBeGreaterThan(0);
    expect(screen.getAllByText('ETH').length).toBeGreaterThan(0);
    expect(screen.getByText('Stellar Lumens')).toBeInTheDocument();
    // Total = 187.58 + 1500 = 1687.58
    expect(screen.getByText('$1,687.58')).toBeInTheDocument();
    // Assets count
    expect(screen.getByText('2')).toBeInTheDocument();
  });

  it('renders a mini chart and conversion panel', () => {
    render(
      <BalanceDisplay
        tokens={tokens}
        historicalData={historicalData}
        conversionRates={conversionRates}
        realTimeUpdates={false}
      />
    );
    expect(screen.getByText('30-Day Performance')).toBeInTheDocument();
    expect(screen.getByText('Token Converter')).toBeInTheDocument();
  });

  it('hides chart and conversion when disabled', () => {
    render(
      <BalanceDisplay
        tokens={tokens}
        historicalData={historicalData}
        conversionRates={conversionRates}
        showChart={false}
        showConversion={false}
        realTimeUpdates={false}
      />
    );
    expect(screen.queryByText('30-Day Performance')).not.toBeInTheDocument();
    expect(screen.queryByText('Token Converter')).not.toBeInTheDocument();
  });

  it('opens the token details modal', () => {
    render(
      <BalanceDisplay
        tokens={tokens}
        historicalData={historicalData}
        conversionRates={conversionRates}
        realTimeUpdates={false}
      />
    );
    fireEvent.click(screen.getAllByText('XLM')[0]);
    expect(screen.getByText('Token Details')).toBeInTheDocument();
    expect(
      screen.getByText('CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2MLQQ4RUE5667W')
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '✕' }));
    expect(screen.queryByText('Token Details')).not.toBeInTheDocument();
  });

  it('converts between tokens', () => {
    render(
      <BalanceDisplay
        tokens={tokens}
        historicalData={historicalData}
        conversionRates={conversionRates}
        realTimeUpdates={false}
      />
    );
    // 1 XLM = 0.15 USD
    expect(screen.getByText(/0\.15 USD/)).toBeInTheDocument();

    const amount = screen.getByPlaceholderText('Amount');
    fireEvent.change(amount, { target: { value: '10' } });
    expect(screen.getByText(/1\.50 USD/)).toBeInTheDocument();

    const fromSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(fromSelect, { target: { value: 'ETH' } });
    expect(screen.getByText(/20,000\.00 USD/)).toBeInTheDocument();
  });

  it('refreshes the portfolio and invokes onRefresh', async () => {
    const onRefresh = jest.fn();
    render(
      <BalanceDisplay
        tokens={tokens}
        historicalData={historicalData}
        conversionRates={conversionRates}
        onRefresh={onRefresh}
        realTimeUpdates={false}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /refresh/i }));
    await waitFor(() => expect(onRefresh).toHaveBeenCalled(), { timeout: 3000 });
  });

  it('renders with mock data when no props are provided', () => {
    render(<BalanceDisplay realTimeUpdates={false} />);
    expect(screen.getAllByText('XLM').length).toBeGreaterThan(0);
    expect(screen.getByText('USD Coin')).toBeInTheDocument();
  });
});
