// put_turn_done — what put_turn says: written (false when the same message was already there),
// masked, cut.
let r = input;
#{ written: r.unchanged != true, masked: r.data.masked, cut: r.data.cut }
