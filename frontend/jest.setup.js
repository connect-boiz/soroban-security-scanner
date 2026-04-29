import '@testing-library/jest-dom';

// Mock react-joyride to avoid issues with DOM animations in tests
jest.mock('react-joyride', () => {
  return function MockJoyride() {
    return <div data-testid="mock-joyride" />;
  };
});
