#pragma once
namespace Billing {
class Box {
public:
    int count;
    class Inner {
    public:
        void leaf();
    };
    Box();
    int size() const;
    virtual void retry() = 0;
};
void helper();
}
namespace Other {
class Box { public: int size() const; };
}
