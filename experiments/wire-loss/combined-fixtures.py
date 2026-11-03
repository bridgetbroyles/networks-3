"""Synthetic simultaneous-anomaly fixtures; never hidden-grader data."""
import copy
from worlds import app, world

apps = [app(100, 3, 101, size=500, rate=0.63, proto='tcp', echo=True), app(101, 8), app(102, 2, 103, size=200, rate=0.16, hosts=list(range(1, 23))), app(103, 7), app(104, 4, 105, size=64, rate=0.02, ttl_ladder=8), app(105, 9), app(106, 12, 107, size=1000, rate=6.0, preset='amazon', on_ms=400, off_ms=600, start_ms=1000), app(107, 13), app(108, 6, 109, size=1000, rate=6.0), app(109, 7), app(110, 6, 111, size=1000, rate=8.0, preset='amazon', on_ms=100, off_ms=1900, start_ms=5030), app(111, 7), app(112, 11, 113), app(113, 12), app(114, 0, 115), app(115, 1), app(116, 10, 117), app(117, 11)]
plants = [dict(kind='side_flow', app=102, to=103, rate_pps=3000.0, size_bytes=64, start_ms=12000, stop_ms=28000, proto='udp'), dict(kind='wire_loss', a=11, b=12, loss_pct=7.0, start_ms=4000, stop_ms=39000), dict(kind='link_capacity', a=12, b=13, steps=[[0, 10], [10000, 4], [27000, 10]]), dict(kind='fail', a=10, b=11, at_ms=12000, for_ms=10000), *[dict(kind='fail', a=a, b=5, at_ms=25000, for_ms=7000) for a in [0, 6, 10]], dict(kind='link_delay', a=1, b=2, a_to_b_us=2000, b_to_a_us=30000), dict(kind='link_delay', a=13, b=14, a_to_b_us=25000, b_to_a_us=25000)]

def text(c, on):
    aa = copy.deepcopy(apps)
    pp = copy.deepcopy(plants)
    gg = [(0, 1), (1, 0)]
    mode = c['mode']

    def get(id):
        return next((x for x in aa if x['id'] == id))
    if mode == 'probe_coupled':
        if on:
            pp.append(dict(kind='fail', a=0, b=1, at_ms=12000, for_ms=10000))
            gg = []
    elif not on:
        if mode == 'capacity_pair':
            next((x for x in pp if x['kind'] == 'link_capacity'))['steps'] = [[0, 10], [10000, 8], [27000, 10]]
        if mode == 'probe_loss':
            next((x for x in pp if x['kind'] == 'wire_loss'))['loss_pct'] = 1.0
        if mode == 'echo':
            get(100)['echo'] = False
        if mode == 'incast_volume':
            get(102)['hosts'] = list(range(1, 10))
        if mode == 'probe_hub':
            pp = [x for x in pp if not (x['kind'] == 'fail' and (x['a'], x['b']) == (10, 5))]
        if mode == 'pulse':
            get(110)['rate_mbps'] = 0.001
        if mode == 'keepalive_only':
            pp.append(dict(kind='fail', a=0, b=1, at_ms=12000, for_ms=10000))
            gg = []
        if mode == 'mapper_ladder':
            get(104).pop('ttl_ladder')
            get(104)['ttl_start'] = 2
        if mode == 'probe_recover':
            pp = [x for x in pp if not (x['kind'] == 'fail' and (x['a'], x['b']) == (10, 11))]
            gg += [(10, 11), (11, 10)]
    t = world(aa, pp, seed=3373)
    for a, b in gg:
        t += f'\n[[harness.keepalive_gap]]\nswitch={a}\ntoward={b}\nstart_ms=12000\nend_ms=22000\n'
    return t
