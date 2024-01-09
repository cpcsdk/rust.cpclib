#include "cpclib-crunchers/extra/Shrinkler4.6NoParityContext/basm_bridge.h"


#include <memory>


#include <cstdio>
#include <cstdlib>
#include <string>
#include <sys/stat.h>

#include "HunkFile.h"
#include "DataFile.h"


#include <cstring>
#include <algorithm>
#include <string>
#include <utility>
#include <algorithm>

using std::make_pair;
using std::max;
using std::min;
using std::pair;
using std::string;

#include "AmigaWords.h"
#include "Pack.h"
#include "RangeDecoder.h"


rust::Vec<unsigned char>  compress_for_basm(rust::Slice<const unsigned char> slice, int iterations, bool log) {
	PackParams params;
	params.iterations = iterations;
	params.length_margin = 1*2;
	params.skip_length = 1000*2;
	params.match_patience = 100*2;
	params.max_same_length = 10*2;

	bool show_progress = true; // TODO set to false ASAP

	vector<unsigned char> data(slice.begin(), slice.end());
	DataFile df(data);
	RefEdgeFactory edge_factory(100000);

	DataFile *crunched = df.crunch(&params, &edge_factory, log);
	if (log) {
		printf("References considered:%8d\n",  edge_factory.max_edge_count);
		printf("References discarded:%9d\n\n", edge_factory.max_cleaned_edges);
	}

	vector<unsigned char> &crunched_data = crunched->data_ref();
	rust::Vec<unsigned char> output;
	output.reserve(crunched_data.size());
	std::copy(crunched_data.begin(), crunched_data.end(), std::back_inserter(output));

	delete crunched;
	return output;
}