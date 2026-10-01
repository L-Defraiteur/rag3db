let checks = [];
for row in input.mana {
    checks.push(#{valid:row.params.actual >= row.params.number,params:row.params});
}
#{checks:checks}
