module.exports = {
  testEnvironment: 'node',
  testMatch: ['**/__tests__/**/*.[jt]s?(x)', '**/?(*.)+(spec|test).[jt]s?(x)'],
  testPathIgnorePatterns: ['/node_modules/', '/frontend/', '/component-library/', '/.next/', '/tests/e2e/', '/tests/e2e copy/', '/tests/accessibility/'],
  collectCoverageFrom: ['src/**/*.js', '!**/node_modules/**'],
  coverageThreshold: {
    global: {
      branches: 80,
      functions: 80,
      lines: 80,
      statements: 80,
    },
  },
};
