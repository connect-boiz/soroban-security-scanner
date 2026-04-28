import { useRef, useState, useEffect, CSSProperties } from 'react';

export interface LazyImageProps {
  src: string;
  webpSrc?: string;
  alt: string;
  width?: number;
  height?: number;
  /**
   * Responsive sizes attribute, e.g. "(max-width: 640px) 100vw, 50vw".
   * Defaults to a sensible mobile-first value.
   */
  sizes?: string;
  /**
   * Optional srcSet for the fallback <img> element.
   * e.g. "image-400.jpg 400w, image-800.jpg 800w, image-1200.jpg 1200w"
   */
  srcSet?: string;
  /**
   * Optional srcSet for the WebP <source> element.
   */
  webpSrcSet?: string;
  className?: string;
  /** How the image should fit its container. Defaults to 'cover'. */
  objectFit?: CSSProperties['objectFit'];
  /** Placeholder color shown while the image loads. */
  placeholderColor?: string;
}

export function LazyImage({
  src,
  webpSrc,
  alt,
  width,
  height,
  sizes = '(max-width: 640px) 100vw, (max-width: 1024px) 50vw, 33vw',
  srcSet,
  webpSrcSet,
  className,
  objectFit = 'cover',
  placeholderColor = '#e5e7eb',
}: LazyImageProps) {
  const [loaded, setLoaded] = useState(false);
  const [inView, setInView] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setInView(true);
          observer.disconnect();
        }
      },
      { rootMargin: '200px' },
    );
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);

  const containerStyle: CSSProperties = {
    position: 'relative',
    overflow: 'hidden',
    width: width ?? '100%',
    height: height ?? 'auto',
    // Maintain aspect ratio when only width is provided
    aspectRatio: width && height ? `${width} / ${height}` : undefined,
    backgroundColor: loaded ? 'transparent' : placeholderColor,
    transition: 'background-color 0.3s',
  };

  const imgStyle: CSSProperties = {
    display: 'block',
    width: '100%',
    height: '100%',
    objectFit,
    opacity: loaded ? 1 : 0,
    transition: 'opacity 0.3s ease-in-out',
  };

  return (
    <div ref={ref} style={containerStyle} className={className}>
      {inView && (
        <picture>
          {/* WebP source — modern browsers prefer this */}
          {(webpSrc || webpSrcSet) && (
            <source
              type="image/webp"
              srcSet={webpSrcSet ?? webpSrc}
              sizes={sizes}
            />
          )}
          {/* Responsive fallback */}
          <img
            src={src}
            srcSet={srcSet}
            alt={alt}
            width={width}
            height={height}
            sizes={sizes}
            loading="lazy"
            decoding="async"
            onLoad={() => setLoaded(true)}
            style={imgStyle}
          />
        </picture>
      )}
    </div>
  );
}

export default LazyImage;
