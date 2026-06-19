module.exports = {
  testEnvironment: 'node',
  testMatch: ['**/__tests__/**/*.[jt]s?(x)', '**/?(*.)+(spec|test).[jt]s?(x)'],
  testPathIgnorePatterns: ['/node_modules/', '/frontend/', '/component-library/', '/.next/', '/tests/e2e/'],
  collectCoverageFrom: ['src/**/*.js', '!**/node_modules/**'],
};
