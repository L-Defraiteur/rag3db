let checks = [];
for row in input.ownership {
    checks.push(#{valid:row.params.actual <= row.params.number,params:row.params});
}
#{checks:checks}
