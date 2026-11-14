# 9. Outline reference for reshaping, and one default image model

Date: 2026-11-14
Status: Accepted (supersedes the generator default in [ADR 0008](0008-cheaper-default-generator.md))

## Context

Reshaping an object with control points used to send the image model the original, a rough warped render of the object, and a sentence built from the point coordinates. The result was often poor: the warp has stretched glass and filled gaps that models reproduce, and the sentence ("widened the base") could disagree with the picture.

Hand-run trials on the perfume photo, same drag, same prompt wording (about $0.03-0.14 each):

| Reference                          | Model                         | Outcome                                                                                   |
| ---------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------------- |
| Magenta outline drawn on the photo | `gemini-3-pro-image`          | Clean, straight-edged result matching the outline                                         |
| Magenta outline                    | `gemini-3.1-flash-image`      | Follows the shape; some ghosted glass and a distorted cap                                 |
| Magenta outline                    | `gemini-3.1-flash-lite-image` | Ignored the shape and copied the guide into the picture, with or without prompt rewording |
| Translucent magenta fill           | `gemini-3-pro-image`          | Worse: kept the old bottle and added extra wings                                          |
| Warped preview (previous approach) | `gemini-3.1-flash-lite-image` | Plausible, smooth flare that follows the preview                                          |

Each is one run on one photo, so treat them as strong hints, not statistics.

## Decisions

- When every pending edit is a control-point reshape, the browser draws each new outline as a thin magenta line with corner dots over the original photo, and sends that as the reference with `preview_kind: "outline"`. The server then uses a fixed prompt that names the edited objects and ignores the point-by-point sentence. Sliders and handles keep the warped preview (`preview_kind: "warped"`, the default).
- The default `generator` is `google/gemini-3.1-flash-image`, the same as `draft_generator`. "Faster draft" is hidden while they are equal. `google/gemini-3-pro-image` stays available in Settings for cleaner results.
- Summaries and change logs still come from the summariser as before.

## Consequences

- Quality on the flash model is acceptable but not as clean as pro; users who need the cleanest reshape can switch the generator in Settings.
- Mixed edits (a slider plus a reshape) use the warped preview, as before.
- A rerun of the benchmark on the golden images is still owed (ADR 0003, ADR 0008).

## Addendum: "understand the object" in every generation prompt
The outline prompt (v2) first asks the model to work out what the object is and how it is built, then to reshape it as a real object of that kind would look, so the result reads as an untouched photograph. On the perfume trial this removed the ghosted glass and distorted cap seen on `gemini-3.1-flash-image` with v1 (one run; Seedream held back and left the bottle unchanged). The same instruction, plus "do not stretch or squash textures; lengthen or shorten by extending or removing material", is now in the warped-preview and no-preview prompts (v2), and the summariser (v2) is told to name the object and describe the change as it would physically look. Not yet measured on the golden images.
