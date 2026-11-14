export const VERTEX_SHADER = `
attribute vec2 position;
void main() { gl_Position = vec4(position, 0.0, 1.0); }
`

/**
 * Two soft blobs drifting on a warm paper (or dark) ground, with the stippled, grainy fade
 * of risograph prints. Everything is analytic, so a frame costs a handful of sines per pixel.
 */
export const FRAGMENT_SHADER = `
precision highp float;
uniform vec2 uResolution;
uniform float uTime;
uniform float uDark;

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

// Distance to a rounded, slowly breathing blob; stretch squashes it into an oval.
float blob(vec2 p, vec2 centre, float radius, vec2 stretch, float lobes, float seed) {
  vec2 q = (p - centre) / stretch;
  float a = atan(q.y, q.x);
  float wobble = 1.0
    + 0.11 * sin(lobes * a + uTime * 0.45 + seed)
    + 0.05 * sin(2.0 * a - uTime * 0.34 + seed * 2.0);
  return (length(q) - radius * wobble) * min(stretch.x, stretch.y);
}

// Smooth union of two distances: joins two lobes into one peanut-shaped body.
float joined(float d1, float d2, float k) {
  float h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
  return mix(d2, d1, h) - k * h * (1.0 - h);
}

vec2 drift(vec2 base, vec2 amplitude, float speed, float seed) {
  return base + amplitude * vec2(
    sin(uTime * speed * 1.5 + seed),
    cos(uTime * speed * 1.25 + seed * 1.7)
  );
}

// Stipple the soft part of an alpha ramp: solid where it is 0 or 1, grainy in between.
float stipple(float alpha, float grain, float amount) {
  float edge = clamp(4.0 * alpha * (1.0 - alpha), 0.0, 1.0);
  return mix(alpha, step(grain, alpha), edge * amount);
}

void main() {
  vec2 uv = gl_FragCoord.xy / uResolution;
  float aspect = uResolution.x / uResolution.y;
  vec2 p = (uv - 0.5) * vec2(aspect, 1.0);
  float grain = hash(gl_FragCoord.xy);

  vec3 paper = vec3(0.973, 0.953, 0.910);
  vec3 night = vec3(0.045, 0.055, 0.105);
  vec3 colour = mix(paper, night, uDark);

  // Ease the colour off behind the headline so it stays readable wherever the blobs drift.
  vec2 middle = (p - vec2(0.0, 0.0)) / vec2(0.28 * aspect, 0.40);
  float calm = mix(0.3, 1.0, smoothstep(0.6, 1.4, length(middle)));
  float strength = calm * mix(1.0, 0.85, uDark);

  // Big salmon blob, top left, with a yellow glow low on its left side.
  vec2 c1 = drift(vec2(-0.27 * aspect, 0.17), vec2(0.08 * aspect, 0.08), 0.10, 1.0);
  float d1 = blob(p, c1, 0.40, vec2(1.0, 1.12), 3.0, 1.0);
  float a1 = (1.0 - smoothstep(-0.015, 0.05, d1)) * strength;
  float body = smoothstep(0.03, -0.12, d1);
  vec3 salmon = mix(
    mix(vec3(0.99, 0.78, 0.64), vec3(0.76, 0.58, 0.98), uDark),
    mix(vec3(0.97, 0.60, 0.44), vec3(0.58, 0.30, 0.93), uDark),
    body
  );
  float glow = exp(-length(p - c1 - vec2(-0.12, -0.2)) * 4.2);
  salmon = mix(salmon, mix(vec3(1.0, 0.92, 0.42), vec3(0.96, 0.42, 0.78), uDark), glow * 0.9);
  colour = mix(colour, salmon, stipple(a1, grain, 0.12));

  // Deep blue peanut, bottom right: dense core, grainy fade to pink and lavender at the top.
  vec2 c2 = drift(vec2(0.24 * aspect, -0.20), vec2(0.08 * aspect, 0.08), 0.08, 2.0);
  vec2 lobe = vec2(0.10, 0.12);
  float d2 = joined(
    blob(p, c2 - lobe, 0.25, vec2(1.0, 1.0), 3.0, 2.0),
    blob(p, c2 + lobe, 0.27, vec2(1.0, 1.0), 2.0, 5.0),
    0.16
  );
  float up = smoothstep(-0.15, 0.30, p.y - c2.y);
  float fade = mix(0.07, 0.30, up);
  float a2 = (1.0 - smoothstep(-fade, 0.03, d2)) * strength;
  float depth = smoothstep(-fade, 0.02, d2) + up * 0.25;
  vec3 deep = mix(vec3(0.16, 0.32, 0.62), vec3(0.05, 0.55, 0.72), uDark);
  vec3 mid = mix(vec3(0.68, 0.68, 0.90), vec3(0.36, 0.78, 0.86), uDark);
  vec3 rim = mix(vec3(0.98, 0.78, 0.74), vec3(0.62, 0.92, 0.84), uDark);
  vec3 blue = mix(deep, mid, clamp(depth * 1.1, 0.0, 1.0));
  blue = mix(blue, rim, smoothstep(0.6, 1.1, depth + up * 0.3));
  colour = mix(colour, blue, stipple(a2, grain, 0.55));

  // Fine film grain over everything.
  colour += (hash(gl_FragCoord.xy + 17.0) - 0.5) * 0.018;
  gl_FragColor = vec4(colour, 1.0);
}
`
