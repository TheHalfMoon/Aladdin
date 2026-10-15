// Windows command-line splitting with the CommandLineToArgvW rules, used to
// compare an engine's OS-reported command line with the host-built argv.
// The program name is taken up to its closing quote (or first whitespace),
// as CommandLineToArgvW does; arguments follow the backslash/quote rules.

export function splitWindowsCommandLine(line: string): string[] {
  const args: string[] = [];
  let i: number;
  if (line.startsWith('"')) {
    const end = line.indexOf('"', 1);
    args.push(end === -1 ? line.slice(1) : line.slice(1, end));
    i = end === -1 ? line.length : end + 1;
  } else {
    const end = line.search(/[ \t]/);
    args.push(end === -1 ? line : line.slice(0, end));
    i = end === -1 ? line.length : end;
  }
  while (i < line.length) {
    while (line[i] === " " || line[i] === "\t") i++;
    if (i >= line.length) break;
    let arg = "";
    let quoted = false;
    while (i < line.length) {
      const ch = line[i];
      if (ch === "\\") {
        let slashes = 0;
        while (line[i] === "\\") {
          slashes++;
          i++;
        }
        if (line[i] === '"') {
          arg += "\\".repeat(Math.floor(slashes / 2));
          if (slashes % 2 === 1) {
            arg += '"';
            i++;
          }
        } else {
          arg += "\\".repeat(slashes);
        }
        continue;
      }
      if (ch === '"') {
        if (quoted && line[i + 1] === '"') {
          arg += '"';
          i += 2;
          continue;
        }
        quoted = !quoted;
        i++;
        continue;
      }
      if ((ch === " " || ch === "\t") && !quoted) break;
      arg += ch;
      i++;
    }
    args.push(arg);
  }
  return args;
}
