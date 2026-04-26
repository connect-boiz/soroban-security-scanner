import { useRef, useState, useEffect } from 'react';

interface LazyImageProps {
  src: string;
  webpSrc?: string;
  alt: string;
  width?: number;
  height?: number;
  sizes?: string;
  className?: string;
}

export default function LazyImage({
  src,
  webpSrc,
  alt,
  width,
  height,
  sizes = '100vw',
  className,
}: LazyImageProps) {
  const [loaded, setLoaded] = useState(false);
  const [inView, setInView] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setInView(true);
          observer.disconnect();
        }
      },
      { rootMargin: '200px' }
    );
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);

  return (
    <div ref={ref} style={{ width, height }} className={className}>
      {inView && (
        <picture>
          {webpSrc && <source srcSet={webpSrc} type="image/webp" sizes={sizes} />}
          <img
            src={src}
            alt={alt}
            width={width}
            height={height}
            sizes={sizes}
            loading="lazy"
            decoding="async"
            onLoad={() => setLoaded(true)}
            style={{ opacity: loaded ? 1 : 0, transition: 'opacity 0.3s' }}
          />
        </picture>
      )}
    </div>
  );
}
