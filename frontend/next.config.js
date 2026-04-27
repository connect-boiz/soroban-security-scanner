/** @type {import('next').NextConfig} */
const nextConfig = {
  // Enable webpack bundle analyzer
  webpack: (config, { isServer }) => {
    if (!isServer) {
      config.optimization.splitChunks = {
        chunks: 'all',
        cacheGroups: {
          default: {
            minChunks: 2,
            priority: -20,
            reuseExistingChunk: true,
          },
          vendor: {
            test: /[\\/]node_modules[\\/]/,
            name: 'vendors',
            priority: -10,
            chunks: 'all',
          },
          react: {
            test: /[\\/]node_modules[\\/](react|react-dom)[\\/]/,
            name: 'react',
            priority: 20,
            chunks: 'all',
          },
        },
      };
    }
    return config;
  },

  // Image optimization
  images: {
    domains: ['localhost'],
    formats: ['image/webp', 'image/avif'],
    minimumCacheTTL: 60 * 60 * 24 * 30, // 30 days
  },

  // Enable compression
  compress: true,

  // Optimize fonts
  optimizeFonts: true,

  // Enable experimental features for performance
  experimental: {
    optimizeCss: true,
    optimizePackageImports: ['@soroban-scanner/ui-components'],
  },

  // Static optimization
  trailingSlash: false,
  
  // Enable SWC minification
  swcMinify: true,

  // Production source maps (disabled for smaller bundles)
  productionBrowserSourceMaps: false,
};

module.exports = nextConfig;
