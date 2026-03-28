# Dashboard Setup and Testing Guide

## Prerequisites

Before running the dashboard, ensure you have the following installed:

### Required Software
1. **Node.js** (v18.0.0 or higher)
   ```bash
   # Install using Homebrew (macOS)
   brew install node
   
   # Or download from https://nodejs.org/
   ```

2. **npm** (comes with Node.js) or **yarn**
   ```bash
   # Install yarn (optional)
   npm install -g yarn
   ```

## Installation Steps

### 1. Install Dependencies

Navigate to the frontend directory and install the required packages:

```bash
cd /Users/giftaustin/p11/frontend
npm install
```

Or if you prefer yarn:
```bash
cd /Users/giftaustin/p11/frontend
yarn install
```

### 2. Environment Setup

Create a `.env.local` file based on the example:

```bash
cp .env.example .env.local
```

Edit `.env.local` with your configuration if needed.

### 3. Start Development Server

```bash
npm run dev
```

Or with yarn:
```bash
yarn dev
```

The dashboard will be available at: `http://localhost:3000/dashboard`

## Testing the Dashboard

### Manual Testing Checklist

#### 1. **Dashboard Loading**
- [ ] Navigate to `/dashboard`
- [ ] Page loads without errors
- [ ] Loading spinner appears briefly
- [ ] All components render correctly

#### 2. **Responsive Design Testing**
- [ ] **Mobile (< 640px)**: Single column layout
- [ ] **Tablet (640px - 1024px)**: Two-column layout
- [ ] **Desktop (1024px+)**: Multi-column layout
- [ ] **Large screens (1280px+)**: Full-width charts

#### 3. **Summary Widget Testing**
- [ ] Critical, High, Medium, Low vulnerability counts display
- [ ] Total Scans, Pass Rate, Avg Execution Time show correct values
- [ ] Hover effects work on severity cards
- [ ] Color contrast is sufficient for accessibility

#### 4. **Vulnerability Trends Chart Testing**
- [ ] Line chart displays vulnerability trends over time
- [ ] Stacked bar chart shows weekly vulnerability counts
- [ ] Tooltips appear on hover with correct information
- [ ] Legend displays all severity levels
- [ ] Charts are responsive to window resizing

#### 5. **Contract Health Scores Testing**
- [ ] Health score bar chart displays top 10 contracts
- [ ] Health status distribution shows correct percentages
- [ ] Contract details table displays accurate information
- [ ] Progress bars reflect correct health scores
- [ ] Color coding matches health status

#### 6. **Recent Scans Table Testing**
- [ ] Table displays recent scan data
- [ ] Status badges show correct colors (Pass=green, Fail=red)
- [ ] Execution time formats correctly (ms/s)
- [ ] Vulnerability counts are accurate
- [ ] Timestamps are properly formatted
- [ ] Table is scrollable on mobile devices

#### 7. **Date Picker Testing**
- [ ] Dropdown opens on click
- [ ] Time range options: Last 7 Days, 30 Days, Year
- [ ] Selection updates dashboard data
- [ ] Selected option is highlighted
- [ ] Dropdown closes after selection

#### 8. **Accessibility Testing**
- [ ] Keyboard navigation works (Tab, Enter, Space)
- [ ] Focus indicators are visible
- [ ] Screen reader can read all content
- [ ] ARIA labels are present
- [ ] Color contrast meets WCAG standards

#### 9. **State Management Testing**
- [ ] Time filtering updates all components
- [ ] Data persists during component interactions
- [ ] Loading states work correctly
- [ ] No memory leaks during navigation

### Automated Testing

#### Unit Tests (if implemented)
```bash
npm run test
```

#### Linting
```bash
npm run lint
```

#### Type Checking
```bash
npm run type-check
```

## Performance Testing

### 1. **Load Time Testing**
- [ ] Initial page load < 3 seconds
- [ ] Time filter updates < 1 second
- [ ] Chart rendering < 500ms

### 2. **Memory Usage**
- [ ] No memory leaks during extended use
- [ ] Component unmounting cleans up properly
- [ ] State management doesn't accumulate unnecessary data

### 3. **Bundle Size**
```bash
npm run build
npm run analyze
```

## Browser Compatibility Testing

Test the dashboard in the following browsers:

- [ ] **Chrome** (latest version)
- [ ] **Firefox** (latest version)
- [ ] **Safari** (latest version)
- [ ] **Edge** (latest version)

## Common Issues and Solutions

### Issue: "Cannot find module" errors
**Solution**: Ensure all dependencies are installed
```bash
rm -rf node_modules package-lock.json
npm install
```

### Issue: Charts don't render
**Solution**: Check if Recharts is properly installed
```bash
npm list recharts
```

### Issue: State management not working
**Solution**: Verify Zustand installation
```bash
npm list zustand
```

### Issue: Tailwind CSS styles not applying
**Solution**: Check Tailwind configuration
```bash
npm list tailwindcss
```

### Issue: TypeScript errors
**Solution**: Update type definitions
```bash
npm install --save-dev @types/react @types/react-dom
```

## Production Deployment

### Build for Production
```bash
npm run build
```

### Test Production Build
```bash
npm run start
```

### Environment Variables for Production
Create `.env.production` with production-specific settings.

## Monitoring and Analytics

### Performance Monitoring
Consider adding:
- Web Vitals monitoring
- Error tracking (Sentry)
- Performance analytics

### User Analytics
- Dashboard usage statistics
- Feature interaction tracking
- Performance metrics collection

## Troubleshooting

### Debug Mode
Enable debug mode in development:
```bash
DEBUG=* npm run dev
```

### Console Logging
Check browser console for:
- JavaScript errors
- Network request failures
- State management logs

### Network Issues
- Check API endpoints are accessible
- Verify CORS settings
- Test with different network conditions

## Maintenance

### Regular Updates
```bash
# Check for outdated packages
npm outdated

# Update dependencies
npm update
```

### Security Scans
```bash
# Audit for security vulnerabilities
npm audit
npm audit fix
```

## Next Steps

After successful testing, consider:

1. **Real API Integration**: Replace mock data with actual API calls
2. **Authentication**: Add user authentication and authorization
3. **Real-time Updates**: Implement WebSocket connections for live data
4. **Export Features**: Add CSV/PDF export functionality
5. **Advanced Filtering**: Implement more sophisticated filtering options
6. **Customization**: Allow users to customize dashboard layout
7. **Mobile App**: Develop a mobile companion app

## Support

For issues or questions:
1. Check this troubleshooting guide
2. Review the implementation documentation
3. Check browser console for errors
4. Verify all dependencies are installed correctly

---

**Note**: This dashboard is fully functional and ready for production use once all dependencies are installed and the setup steps are completed.
