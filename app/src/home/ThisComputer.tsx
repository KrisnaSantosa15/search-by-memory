import { BrainCircuit, Cpu, Keyboard, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import { spoken } from "../onboarding/HotkeyStep";
import type { ModelState } from "../Settings";

/** What search runs on, here: the model, where it runs, the keys, and that it all stays local. */
export function ThisComputer({ model, hotkey }: { model: ModelState; hotkey: string | null }) {
  return (
    <section
      aria-labelledby="this-computer"
      className="flex flex-col gap-3 rounded-2xl border border-line bg-panel/40 p-5"
    >
      <h2 id="this-computer" className="text-sm font-medium">
        On this computer
      </h2>
      <dl className="flex flex-col gap-3">
        <Fact Icon={BrainCircuit} term="Reading model">
          EmbeddingGemma 2
          <span className="block text-xs font-normal text-muted">Works across many languages, Indonesian too</span>
        </Fact>
        <Fact Icon={Cpu} term="Runs on">
          {runsOn(model)}
        </Fact>
        {hotkey && (
          <Fact Icon={Keyboard} term="Search keys">
            {spoken(hotkey)}
          </Fact>
        )}
        <Fact Icon={ShieldCheck} term="Privacy">
          Nothing leaves this computer
        </Fact>
      </dl>
    </section>
  );
}

function runsOn(model: ModelState): string {
  if (model.state === "loading") return "Not loaded yet";
  if (model.state === "failed") return "Not loaded";
  return model.device === "cpu" ? "Processor" : "Graphics card";
}

function Fact(props: { Icon: typeof Cpu; term: string; children: ReactNode }) {
  const { Icon } = props;
  return (
    <div className="flex items-start gap-3">
      <span className="grid size-8 shrink-0 place-items-center rounded-lg border border-line bg-raised text-muted">
        <Icon aria-hidden className="size-4" strokeWidth={1.75} />
      </span>
      <span className="flex min-w-0 flex-col">
        <dt className="text-xs text-muted">{props.term}</dt>
        <dd className="text-sm font-medium">{props.children}</dd>
      </span>
    </div>
  );
}
