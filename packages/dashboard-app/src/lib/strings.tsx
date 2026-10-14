import { Code } from "@/components/text/code";
import { Kbd } from "@/components/ui/keystrokeInput";

// Wraps every other part in `Wrap`: the even parts if the string starts with
// the delimiter, the odd parts otherwise.
function wrapAlternateParts(
  parts: string[],
  startsWithDelimiter: boolean,
  Wrap: typeof Code | typeof Kbd,
  className?: string,
) {
  const wrappedParity = startsWithDelimiter ? 0 : 1;
  return (
    <span>
      {parts.map((s, i) =>
        i % 2 === wrappedParity ? (
          <Wrap key={i} className={className}>
            {s}
          </Wrap>
        ) : (
          <span key={i}>{s}</span>
        ),
      )}
    </span>
  );
}

export function parseBracketsToKbd(string: string, className?: string) {
  if (string.indexOf("[") === -1) return <span>{string}</span>;

  // This *works* but is so hacky that it feels awful and some strings have both backticks *and* square brackets...
  // TODO: switch to regex-based solution
  const strArray = string.split("[").join("*").split("]").join("*").split("*");
  return wrapAlternateParts(strArray, string.charAt(0) === "[", Kbd, className);
}

export function parseBackticksToCode(string: string, className?: string) {
  if (string.indexOf("`") === -1) return <span>{string}</span>;

  return wrapAlternateParts(
    string.split("`"),
    string.charAt(0) === "`",
    Code,
    className,
  );
}
