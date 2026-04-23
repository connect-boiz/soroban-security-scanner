import { useState, useEffect } from 'react';
import { Bounty } from '@/types/bounty';

interface CountdownTimerProps {
  deadline: Date;
  className?: string;
}

export const CountdownTimer: React.FC<CountdownTimerProps> = ({ deadline, className = '' }) => {
  const [timeLeft, setTimeLeft] = useState({
    days: 0,
    hours: 0,
    minutes: 0,
    seconds: 0,
    expired: false
  });

  useEffect(() => {
    const calculateTimeLeft = () => {
      const difference = deadline.getTime() - new Date().getTime();
      
      if (difference > 0) {
        setTimeLeft({
          days: Math.floor(difference / (1000 * 60 * 60 * 24)),
          hours: Math.floor((difference / (1000 * 60 * 60)) % 24),
          minutes: Math.floor((difference / 1000 / 60) % 60),
          seconds: Math.floor((difference / 1000) % 60),
          expired: false
        });
      } else {
        setTimeLeft(prev => ({ ...prev, expired: true }));
      }
    };

    calculateTimeLeft();
    const timer = setInterval(calculateTimeLeft, 1000);

    return () => clearInterval(timer);
  }, [deadline]);

  if (timeLeft.expired) {
    return (
      <div className={`text-red-600 font-semibold ${className}`}>
        ⏰ Expired
      </div>
    );
  }

  return (
    <div className={`flex items-center space-x-2 ${className}`}>
      <span className="text-gray-600">⏱️</span>
      {timeLeft.days > 0 && (
        <span className="font-mono text-sm">
          {timeLeft.days}d {timeLeft.hours}h {timeLeft.minutes}m {timeLeft.seconds}s
        </span>
      )}
      {timeLeft.days === 0 && timeLeft.hours > 0 && (
        <span className="font-mono text-sm">
          {timeLeft.hours}h {timeLeft.minutes}m {timeLeft.seconds}s
        </span>
      )}
      {timeLeft.days === 0 && timeLeft.hours === 0 && (
        <span className="font-mono text-sm text-orange-600 font-semibold">
          {timeLeft.minutes}m {timeLeft.seconds}s
        </span>
      )}
    </div>
  );
};
