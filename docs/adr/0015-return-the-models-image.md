# ADR 0015: Return the image model's output as the result

Status: accepted (supersedes the compositing decisions in [ADR 0006](0006-apply-pipeline.md))

## Context

Apply used to blend the model's image into the original only inside a rectangle around the edited object, after colour-matching it, so every other pixel stayed byte-identical. The rectangle came from the object's box and where the preview put it.

Image models do not keep to the preview's size. Enlarging a burger to 150% produced a model image where the whole burger, with its domed bun, was about twice as tall as the preview showed and reached well past the rectangle. The raw output was complete and natural; the blend cut it to the rectangle, leaving straight edges, a flat top and a dark patch where the model's background differed from the photo. Saving the raw output with the job (`jobs/<id>/generated`) is what showed this.

## Decision

The result is the model's image, resized to the original's resolution. The region boxes from the browser, the padded rectangles, colour matching and feathered blending are removed, along with `regions` on `EditRequest`.

## Consequences

- Edited objects are never clipped, whatever size the model draws them.
- Pixels outside the object can differ slightly from the original, because the model re-renders the whole frame. The generator prompts still say everything else must stay identical; the quality of that depends on the model.
- Output sharpness is the model's (about 1-1.4 megapixels), resized up to the original's resolution.
- Less code: the compositing and object colour modules and the client-side region calculation are gone.
- `jobs/<id>/generated` and `jobs/<id>/preview.jpg` remain, so a bad result can be traced to the model or to our prompt.
