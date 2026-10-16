import { Cpu, Gpu, MemoryStick } from "lucide-react";
import { useEffect, useState } from "react";

/** How busy this computer is and how much of that is this app; see `DeviceUsage` in `src-tauri`. */
export type DeviceUsage = {
  /** `null` until a second sample can say how busy the processor was. */
  cpu: { percent: number; ownPercent: number } | null;
  processor: { name: string; cores: number; threads: number; ghz: number } | null;
  /** Bytes of physical memory. */
  memory: { used: number; total: number; own: number } | null;
  gpus: {
    name: string;
    percent: number;
    ownPercent: number;
    /** Bytes of video memory. */
    memoryUsed: number;
    memoryTotal: number;
    ownMemory: number;
  }[];
};

/** How often the computer is sampled while the panel is open. */
const SAMPLE_EVERY_MS = 2_000;

const GIB = 2 ** 30;
const oneDecimal = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

function gigabytes(bytes: number): string {
  if (bytes < GIB / 10) return `${Math.round(bytes / 2 ** 20)} MB`;
  return `${oneDecimal.format(bytes / GIB)} GB`;
}

const percent = (value: number) => `${Math.round(value)}%`;

/** The newest sample of the computer, taken every few seconds while `open`. */
function useDeviceUsage(deviceUsage: () => Promise<DeviceUsage>, open: boolean) {
  const [usage, setUsage] = useState<DeviceUsage | null>(null);
  useEffect(() => {
    if (!open) return;
    let current = true;
    const sample = () =>
      deviceUsage().then(
        (usage) => current && setUsage(usage),
        () => {},
      );
    sample();
    const timer = setInterval(sample, SAMPLE_EVERY_MS);
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, [deviceUsage, open]);
  return usage;
}

/** This computer's processor, memory and graphics cards, live, with this app's share of each. */
export function DevicePanel(props: { deviceUsage: () => Promise<DeviceUsage>; open: boolean }) {
  const usage = useDeviceUsage(props.deviceUsage, props.open);
  if (!usage) {
    return (
      <div aria-hidden className="flex flex-col gap-3 rounded-xl border border-line bg-canvas/50 p-3">
        {[0, 1, 2].map((row) => (
          <span key={row} className="h-8 rounded-md bg-hover/60 motion-safe:animate-pulse" />
        ))}
      </div>
    );
  }
  const { cpu, processor, memory, gpus } = usage;
  return (
    <div className="flex flex-col gap-3.5 rounded-xl border border-line bg-canvas/50 p-3">
      <Metric
        Icon={Cpu}
        label="Processor"
        value={cpu ? percent(cpu.percent) : "Measuring…"}
        detail={processor?.name}
        meta={
          processor && `${processor.cores} cores, ${processor.threads} threads, ${processor.ghz} GHz`
        }
        all={cpu?.percent ?? 0}
        own={cpu?.ownPercent ?? 0}
        ownLabel={cpu && percent(cpu.ownPercent)}
      />
      {memory && (
        <Metric
          Icon={MemoryStick}
          label="Memory"
          value={`${oneDecimal.format(memory.used / GIB)} of ${oneDecimal.format(memory.total / GIB)} GB`}
          all={(memory.used * 100) / memory.total}
          own={(memory.own * 100) / memory.total}
          ownLabel={gigabytes(memory.own)}
        />
      )}
      {gpus.map((gpu) => (
        <Metric
          key={gpu.name}
          Icon={Gpu}
          label={gpu.name}
          value={percent(gpu.percent)}
          meta={
            gpu.memoryTotal > 0
              ? `${oneDecimal.format(gpu.memoryUsed / GIB)} of ${oneDecimal.format(gpu.memoryTotal / GIB)} GB video memory`
              : undefined
          }
          all={gpu.percent}
          own={gpu.ownPercent}
          ownLabel={`${percent(gpu.ownPercent)}${gpu.ownMemory > 0 ? `, ${gigabytes(gpu.ownMemory)}` : ""}`}
        />
      ))}
    </div>
  );
}

/**
 * One part of the computer: how busy it is, drawn as one bar whose bright
 * part is this app and whose dim part is everything else.
 */
function Metric(props: {
  Icon: typeof Cpu;
  label: string;
  value: string;
  detail?: string;
  meta?: string | null;
  /** Share in use by everything, from 0 to 100. */
  all: number;
  /** Share in use by this app, from 0 to 100. */
  own: number;
  ownLabel?: string | null;
}) {
  const { Icon } = props;
  const all = Math.min(Math.max(props.all, 0), 100);
  const own = Math.min(Math.max(props.own, 0), all);
  return (
    <div role="group" aria-label={props.label} className="flex min-w-0 flex-col gap-1.5">
      <div className="flex items-center gap-2 text-xs">
        <Icon aria-hidden className="size-3.5 shrink-0 text-muted" strokeWidth={1.75} />
        <span className="min-w-0 truncate font-medium" title={props.label}>
          {props.label}
        </span>
        <span className="ml-auto shrink-0 tabular-nums text-muted">{props.value}</span>
      </div>
      {(props.detail || props.meta) && (
        <div className="flex flex-col text-[10.5px] leading-[14px] text-muted">
          {props.detail && (
            <span className="truncate" title={props.detail}>
              {props.detail}
            </span>
          )}
          {props.meta && <span className="truncate tabular-nums">{props.meta}</span>}
        </div>
      )}
      <div aria-hidden className="relative h-1.5 overflow-hidden rounded-full bg-hover">
        <div
          className="absolute inset-y-0 left-0 rounded-full bg-accent/35 transition-[width] duration-500 motion-reduce:transition-none"
          style={{ width: `${all}%` }}
        />
        <div
          className="absolute inset-y-0 left-0 rounded-full bg-accent transition-[width] duration-500 motion-reduce:transition-none"
          style={{ width: `${own}%` }}
        />
      </div>
      {props.ownLabel && (
        <span className="flex items-center gap-1.5 text-[10.5px] leading-[14px] text-muted">
          <span aria-hidden className="size-1.5 shrink-0 rounded-full bg-accent" />
          <span className="tabular-nums">Search by Memory {props.ownLabel}</span>
        </span>
      )}
    </div>
  );
}
