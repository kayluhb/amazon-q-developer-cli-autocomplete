import autocompleteDemo from "@assets/images/autocomplete_demo.gif";
import { Link } from "@/components/ui/link";
import { Terminal } from "@/components/ui/terminal";

export default function Page() {
  return (
    <div className="flex flex-col items-start gap-4">
      <div className="flex flex-col">
        <h1 className="text-2xl font-bold dark:text-zinc-400">
          Getting started
        </h1>
      </div>
      <div className="flex flex-col gap-2 w-full border bg-zinc-50 border-zinc-200 dark:bg-zinc-900 dark:border-zinc-600 rounded-lg p-4 mb-4 overflow-x-auto">
        <h2 className="font-bold font-heading tracking-tight text-lg dark:text-zinc-300">
          Launch a new shell session to start using autocomplete!
        </h2>
        <ol className="flex flex-col gap-0.5 text-zinc-600 dark:text-zinc-400">
          <li>
            <span className="text-sm flex items-baseline gap-1">
              <span className="font-semibold">Not working?</span>
              <span>Check out</span>
              <Link to="/help" variant="blue">
                Help & support
              </Link>
            </span>
          </li>
          <li>
            <span className="text-sm">
              <span className="font-semibold">Want to customize settings?</span>{" "}
              Use the tabs in the sidebar.
            </span>
          </li>
        </ol>
      </div>
      <Terminal tabNames={["Autocomplete"]}>
        <Terminal.Tab>
          <img src={autocompleteDemo} alt="autocomplete demo" />
        </Terminal.Tab>
      </Terminal>
    </div>
  );
}
