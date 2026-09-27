let checks = [];
for row in input.low_mana { checks.push(#{valid:row.params.actual >= row.params.number,params:row.params}); }
#{checks:checks}
