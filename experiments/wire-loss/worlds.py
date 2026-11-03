import json

def app(id, sw, peer=None, size=200, rate=0.02, proto='udp', hosts=None, **kw):
    d = {'id': id, 'switch': sw, 'prefix': f'10.{id - 99}.0.0/24', 'name': f'Synthetic {id}'}
    if peer is not None:
        d.update(preset='fortnite', peer=peer, size_bytes=size, rate_mbps=rate, proto=proto)
    if hosts:
        d['hosts'] = hosts
    d.update(kw)
    return d

def world(apps, plants=(), seed=3303):
    lines = ['[world]', 'name="remaining-claim-validation"', f'seed={seed}', 'family="harness"', 'version=1', '[params]', 'link_capacity_mbps=10', 'link_latency_us=500', 'queue_capacity_bytes=40000', 'warmup_ms=3000', 'duration_ms=40000', 'delivery_floor_pct=0.0', 'packet_size_bytes=1000', 'control_delay_us=2000']
    for s in range(15):
        lines += ['[[switch]]', f'id={s}']
    edges = [(r * 5 + c, r * 5 + c + 1) for r in range(3) for c in range(4)] + [(r * 5 + c, (r + 1) * 5 + c) for r in range(2) for c in range(5)]
    for a, b in edges:
        lines += ['[[link]]', f'a={a}', f'b={b}']
    for a in apps:
        lines += ['[[app]]'] + [f'{k}={json.dumps(v)}' for k, v in a.items()]
    lines += ['[workload]', 'kind="mixed"', 'size_bytes=1000', '[observe]', 'punt_events_per_s=80', 'punt_backlog=64', 'export_bytes_per_s=16000', 'export_burst_bytes=8000', '[harness]', 'keepalive_ms=100', 'log_cap_bytes=24576']
    for p in plants:
        lines += ['[[plant]]'] + [f'{k}={json.dumps(v)}' for k, v in p.items()]
    return '\n'.join(lines) + '\n'
