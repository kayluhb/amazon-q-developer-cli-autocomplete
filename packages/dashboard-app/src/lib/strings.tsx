import { Code } from "@/components/text/code";
import { Kbd } from "@/components/ui/keystrokeInput";

// Wraps the odd parts in `Wrap`. Splitting on the delimiter always puts the
// delimited text at the odd indices: "`a` b".split("`") is ["", "a", " b"].
function wrapAlternateParts(
  parts: string[],
  Wrap: typeof Code | typeof Kbd,
  className?: string,
) {
  return (
    <span>
      {parts.map((part, index) =>
        index % 2 === 1 ? (
          <Wrap key={index} className={className}>
            {part}
          </Wrap>
        ) : (
          <span key={index}>{part}</span>
        ),
      )}
    </span>
  );
}

export function parseBracketsToKbd(string: string, className?: string) {
  if (!string.includes("[")) return <span>{string}</span>;

  // This *works* but is so hacky that it feels awful and some strings have both backticks *and* square brackets...
  // TODO: switch to regex-based solution
  const strArray = string.split("[").join("*").split("]").join("*").split("*");
  return wrapAlternateParts(strArray, Kbd, className);
}

export function parseBackticksToCode(string: string, className?: string) {
  if (!string.includes("`")) return <span>{string}</span>;

  return wrapAlternateParts(string.split("`"), Code, className);
}
