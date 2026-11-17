// Private read-only typed transport. No index, embedding or writer API.
#include "rag3db.hpp"
#include <cstdint>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
using namespace rag3db::main;
using rag3db::common::Value;
using rag3db::common::NestedVal;
using rag3db::common::LogicalTypeID;
static void u64(std::string& out,uint64_t n){for(unsigned i=0;i<8;i++)out.push_back(char((n>>(8*i))&255));}
static void bytes(std::string& out,const std::string& value){u64(out,value.size());out+=value;}
static void value(std::string& out,const Value& v,unsigned depth=0){
 if(depth>64)throw std::runtime_error("typed nesting exceeds bound");
 bytes(out,v.getDataType().toString());out.push_back(v.isNull()?'0':'1');if(v.isNull())return;
 auto type=v.getDataType().getLogicalTypeID();
 if(type==LogicalTypeID::STRING||type==LogicalTypeID::BLOB){out.push_back('s');bytes(out,v.getValue<std::string>());}
 else if(type==LogicalTypeID::FLOAT){out.push_back('f');float n=v.getValue<float>();uint32_t bits;std::memcpy(&bits,&n,4);for(unsigned i=0;i<4;i++)out.push_back(char((bits>>(8*i))&255));}
 else if(type==LogicalTypeID::DOUBLE){out.push_back('d');double n=v.getValue<double>();uint64_t bits;std::memcpy(&bits,&n,8);u64(out,bits);}
 else if(type==LogicalTypeID::LIST||type==LogicalTypeID::ARRAY||type==LogicalTypeID::STRUCT||type==LogicalTypeID::MAP){out.push_back('a');auto n=NestedVal::getChildrenSize(&v);u64(out,n);for(uint32_t i=0;i<n;i++)value(out,*NestedVal::getChildVal(&v,i),depth+1);}
 else {out.push_back('n');bytes(out,v.toString());}
}
static void write_u64(uint64_t n){std::string out;u64(out,n);std::cout.write(out.data(),out.size());}
static void emit(const std::string& row){write_u64(row.size());std::cout.write(row.data(),row.size());if(!std::cout)throw std::runtime_error("typed stream write refused");}
int main(int argc,char** argv){try{
 if(argc==2&&std::string(argv[1])=="--self-test"){
  std::cout.write("R3TYP002",8);write_u64(3);write_u64(1);std::string row;
  value(row,Value(std::string("a\0|\nb",5)));value(row,Value(float(1.25)));value(row,Value(std::string("")));emit(row);return 0;
 }
 if(argc!=3)throw std::runtime_error("expected BASE READ_QUERY");
 std::string sql=argv[2];if(sql.rfind("MATCH ",0)!=0&&sql.rfind("CALL ",0)!=0)throw std::runtime_error("not a read-query prefix");
 SystemConfig cfg;cfg.readOnly=true;cfg.bufferPoolSize=1ull<<30;Database db(argv[1],cfg);Connection c(&db);
 auto b=c.query("CALL BUILD_ID() RETURN *");if(!b->isSuccess())throw std::runtime_error("BUILD_ID refused");while(b->hasNext())std::cerr<<"BUILD_ID\t"<<b->getNext()->toString();
 auto r=c.query(sql);if(!r->isSuccess())throw std::runtime_error(r->getErrorMessage());
 auto columns=r->getNumColumns();auto rows=r->getNumTuples();std::cout.write("R3TYP002",8);write_u64(columns);write_u64(rows);uint64_t seen=0;
 while(r->hasNext()){auto tuple=r->getNext();std::string row;for(size_t i=0;i<columns;i++)value(row,*tuple->getValue(i));emit(row);seen++;}
 if(seen!=rows||!std::cout)throw std::runtime_error("typed count differs");return 0;
 }catch(const std::exception& e){std::cerr<<"TYPED_READ_REFUSED\t"<<e.what()<<'\n';return 1;}}
