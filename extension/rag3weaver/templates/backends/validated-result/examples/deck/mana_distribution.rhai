let checks = [];
for row in input.low_mana {
    checks.push(#{valid:row.params.actual >= row.params.number || row.params.actual < input.policy.minimum_sources,params:row.params});
}
#{checks:checks}
